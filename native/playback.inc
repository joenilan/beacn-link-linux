/* Opt-in retained eight-channel adapter. Included after the existing private
 * core/registry/creation owners. No numeric-ID-selected destroy operation. */
#include <spa/param/audio/format-utils.h>
#include <spa/param/port-config.h>

struct playback_refs {
  struct receipt node, links[2];
  struct receipt ports[8], monitors[2];
};

static void playback_node_info(void *data, const struct pw_node_info *info) {
  struct created *c = data;
  c->info = true;
  c->state = info->state;
  if (info->state == PW_NODE_STATE_ERROR)
    c->error = true;
}
static const struct pw_node_events playback_node_events = {
    .version = PW_VERSION_NODE_EVENTS, .info = playback_node_info};

static struct created *playback_created(struct session *s, const char *factory,
                                       const char *type, uint32_t version,
                                       struct pw_properties *props,
                                       const char *owner, bool node,
                                       struct object *out, struct object *in) {
  struct created *c = calloc(1, sizeof(*c));
  if (!c)
    return NULL;
  c->s = s;
  snprintf(c->owner, sizeof(c->owner), "%s", owner);
  if (!node) {
    c->out_node = (uint32_t)num(prop(out, PW_KEY_NODE_ID));
    c->out_port = out->id;
    c->in_node = (uint32_t)num(prop(in, PW_KEY_NODE_ID));
    c->in_port = in->id;
    pw_properties_setf(props, PW_KEY_LINK_OUTPUT_PORT, "%u", out->id);
    pw_properties_setf(props, PW_KEY_LINK_INPUT_PORT, "%u", in->id);
  }
  c->proxy = pw_core_create_object(s->core, factory, type, version,
                                   &props->dict, 0);
  if (!c->proxy) {
    free(c);
    return NULL;
  }
  /* From this point every outcome is released by bcl_close's retained ledger,
   * including an uncertain create without a global ID or acknowledgment. */
  c->next = s->created;
  s->created = c;
  s->pending = c;
  c->local_id = pw_proxy_get_id(c->proxy);
  pw_proxy_add_listener(c->proxy, &c->listener, &created_events, c);
  pw_proxy_add_object_listener(c->proxy, &c->info_listener,
                               node ? (const void *)&playback_node_events
                                    : (const void *)&created_link_events,
                               c);
  return c;
}
static int playback_ack(struct session *s, struct created *c, bool node,
                        struct receipt *result) {
  int64_t deadline = millis() + 5000;
  do {
    if (barrier(s) < 0 || barrier(s) < 0 || c->removed || c->error)
      return -1;
    struct object *o = c->bound ? find(s, c->id, node ? NODE : LINK) : NULL;
    if (o && o->info && c->info && serial(o) &&
        spa_streq(prop(o, OWNER), c->owner) &&
        (node || (o->state == PW_LINK_STATE_ACTIVE &&
                  c->state == PW_LINK_STATE_ACTIVE &&
                  o->out_node == c->out_node && o->out_port == c->out_port &&
                  o->in_node == c->in_node && o->in_port == c->in_port))) {
      c->serial = serial(o);
      *result = (struct receipt){c->id, c->serial};
      s->pending = NULL;
      return 0;
    }
    if (pw_loop_iterate(pw_main_loop_get_loop(s->loop), 20) < 0)
      return -1;
  } while (millis() < deadline);
  fprintf(stderr, "native playback acknowledgment timed out: node=%d bound=%d info=%d state=%d\n",
          node, c->bound, c->info, c->state);
  return -1;
}
static bool playback_source(struct session *s, const struct receipt *source,
                            const char *owner, struct object **outputs) {
  struct object *n = find(s, source->id, NODE);
  if (!n || !n->info || serial(n) != source->serial ||
      !spa_streq(prop(n, PW_KEY_MEDIA_CLASS), "Audio/Sink") ||
      !spa_streq(prop(n, "node.virtual"), "true") ||
      !spa_streq(prop(n, "beacn.test.token"), owner) ||
      prop(n, PW_KEY_DEVICE_ID))
    return false;
  struct object *inputs[2] = {NULL, NULL};
  outputs[0] = outputs[1] = NULL;
  unsigned count = 0;
  for (size_t i = 0; i < s->count; i++) {
    struct object *o = s->objects[i];
    if (!o->present || o->kind != PORT ||
        num(prop(o, PW_KEY_NODE_ID)) != n->id)
      continue;
    const char *ch = prop(o, PW_KEY_AUDIO_CHANNEL);
    int side = spa_streq(ch, "FL") ? 0 : spa_streq(ch, "FR") ? 1 : -1;
    if (!o->info || !serial(o) || side < 0)
      { fprintf(stderr,"native playback monitor refused: direction=%d channel=%s info=%d\n",o->direction,ch?ch:"absent",o->info); return false; }
    struct object **slot = o->direction == SPA_DIRECTION_INPUT ? &inputs[side]
                                                               : &outputs[side];
    if (*slot || (o->direction != SPA_DIRECTION_INPUT &&
                  !spa_streq(prop(o, "port.monitor"), "true")))
      return false;
    *slot = o;
    count++;
  }
  if (count != 4 || !inputs[0] || !inputs[1] || !outputs[0] || !outputs[1])
    return false;
  unsigned routed[2] = {0, 0};
  for (size_t i = 0; i < s->count; i++) {
    struct object *l = s->objects[i];
    if (!l->present || l->kind != LINK || l->in_node != n->id)
      continue;
    int side = l->in_port == inputs[0]->id ? 0 :
               l->in_port == inputs[1]->id ? 1 : -1;
    if (side < 0 || !l->info || !serial(l) ||
        l->state < PW_LINK_STATE_PAUSED || !spa_streq(prop(l, OWNER), owner))
      return false;
    routed[side]++;
  }
  return routed[0] == 1 && routed[1] == 1;
}
static bool playback_ports(struct session *s, const struct receipt *node,
                           struct object **inputs) {
  struct object *n = find(s, node->id, NODE);
  if (!n || serial(n) != node->serial || !n->info)
    return false;
  for (int i = 0; i < 8; i++)
    inputs[i] = NULL;
  unsigned count = 0;
  for (size_t i = 0; i < s->count; i++) {
    struct object *p = s->objects[i];
    if (!p->present || p->kind != PORT ||
        num(prop(p, PW_KEY_NODE_ID)) != node->id)
      continue;
    const char *ch = prop(p, PW_KEY_AUDIO_CHANNEL);
    if (!p->info || !serial(p) || p->direction != SPA_DIRECTION_INPUT || !ch ||
        strlen(ch) != 4 || strncmp(ch, "AUX", 3) || ch[3] < '0' || ch[3] > '7') {
      fprintf(stderr,"native playback input refused: direction=%d channel=%s info=%d\n",p->direction,ch?ch:"absent",p->info);
      return false;
    }
    unsigned side = (unsigned)(ch[3] - '0');
    if (inputs[side])
      return false;
    inputs[side] = p;
    count++;
  }
  return count == 8;
}
int bcl_playback_create(struct session *s, const struct receipt *source,
                        const char *owner, int pair, int card,
                        struct playback_refs *result) {
  if (!s || s->catalog_only || s->fatal || s->pending || s->created ||
      !source || !result || !label(owner) || pair < 1 || pair > 4 ||
      card < -1 || card > 255 || barrier(s) < 0 || barrier(s) < 0)
    return -1;
  struct object *outputs[2];
  if (!playback_source(s, source, owner, outputs))
    return -10;
  /* Never fan out or replace any existing monitor link, even with our tag. */
  for (size_t i = 0; i < s->count; i++) {
    struct object *l = s->objects[i];
    if (l->present && l->kind == LINK &&
        (l->out_port == outputs[0]->id || l->out_port == outputs[1]->id))
      return -11;
  }
  int fault = s->playback_fault;
  s->playback_fault = 0;
  if (fault == 1)
    return -1;
  char name[80];
  snprintf(name, sizeof(name), "linx-playback-%u-%u", (unsigned)getpid(), source->id);
  struct pw_properties *p = pw_properties_new(
      "factory.name", card < 0 ? "support.null-audio-sink" : "api.alsa.pcm.sink",
      PW_KEY_NODE_NAME, name, PW_KEY_MEDIA_CLASS, "Audio/Sink",
      PW_KEY_NODE_AUTOCONNECT, "false", PW_KEY_OBJECT_LINGER, "false",
      "node.driver", "true", "node.pause-on-idle", "true",
      "audio.rate", "96000", "audio.allowed-rates", "[ 96000 ]",
      "audio.format", "F32LE", "audio.channels", "8",
      "audio.position", "[ AUX0 AUX1 AUX2 AUX3 AUX4 AUX5 AUX6 AUX7 ]",
      "beacn.test.token", owner, OWNER, owner, NULL);
  if (!p)
    return -1;
  if (card < 0)
    pw_properties_set(p, "node.virtual", "true");
  else {
    pw_properties_setf(p, "api.alsa.path", "hw:%d,0", card);
    pw_properties_set(p, "api.alsa.use-chmap", "false");
  }
  struct created *node = playback_created(s, "adapter", PW_TYPE_INTERFACE_Node,
                                          PW_VERSION_NODE, p, owner, true, NULL, NULL);
  pw_properties_free(p);
  if (!node || playback_ack(s, node, true, &result->node) < 0)
    return -20;
  uint8_t fbuf[1024], buf[2048];
  struct spa_pod_builder fb = SPA_POD_BUILDER_INIT(fbuf, sizeof(fbuf));
  struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buf, sizeof(buf));
  struct spa_audio_info_raw info = {.format = SPA_AUDIO_FORMAT_F32P,
                                   .rate = 96000, .channels = 8};
  for (int i = 0; i < 8; i++)
    info.position[i] = SPA_AUDIO_CHANNEL_AUX0 + i;
  struct spa_pod *format = spa_format_audio_raw_build(&fb, SPA_PARAM_Format, &info);
  struct spa_pod *config = spa_pod_builder_add_object(
      &b, SPA_TYPE_OBJECT_ParamPortConfig, SPA_PARAM_PortConfig,
      SPA_PARAM_PORT_CONFIG_direction, SPA_POD_Id(SPA_DIRECTION_INPUT),
      SPA_PARAM_PORT_CONFIG_mode, SPA_POD_Id(SPA_PARAM_PORT_CONFIG_MODE_dsp),
      SPA_PARAM_PORT_CONFIG_monitor, SPA_POD_Bool(false),
      SPA_PARAM_PORT_CONFIG_control, SPA_POD_Bool(false),
      SPA_PARAM_PORT_CONFIG_format, SPA_POD_Pod(format));
  if (!config || pw_node_set_param((struct pw_node *)node->proxy,
                                   SPA_PARAM_PortConfig, 0, config) < 0 ||
      barrier(s) < 0 || barrier(s) < 0)
    return -21;
  struct object *inputs[8];
  if (!playback_ports(s, &result->node, inputs) || fault == 2)
    return -22;
  for (int i = 0; i < 8; i++)
    result->ports[i] = (struct receipt){inputs[i]->id, serial(inputs[i])};
  for (int i = 0; i < 2; i++)
    result->monitors[i] = (struct receipt){outputs[i]->id, serial(outputs[i])};
  for (int i = 0; i < 2; i++) {
    if (!playback_source(s, source, owner, outputs) ||
        !playback_ports(s, &result->node, inputs))
      return -23;
    p = pw_properties_new(PW_KEY_OBJECT_LINGER, "false", OWNER, owner, NULL);
    if (!p)
      return -24;
    struct created *link = playback_created(s, "link-factory", PW_TYPE_INTERFACE_Link,
        PW_VERSION_LINK, p, owner, false, outputs[i], inputs[(pair-1)*2+i]);
    pw_properties_free(p);
    if (!link || (fault == 4 && i == 1) ||
        playback_ack(s, link, false, &result->links[i]) < 0 ||
        (fault == 3 && i == 0))
      return -25;
  }
  return 0;
}
int bcl_playback_check(struct session *s, const struct receipt *source,
                       const char *owner, int pair, const struct playback_refs *refs) {
  if (!s || !source || !refs || s->catalog_only || s->fatal || s->pending || !label(owner) ||
      pair < 1 || pair > 4 || barrier(s) < 0 || barrier(s) < 0)
    return -1;
  struct object *outputs[2], *inputs[8];
  if (!playback_source(s, source, owner, outputs) ||
      !playback_ports(s, &refs->node, inputs))
    return -1;
  struct object *node = find(s, refs->node.id, NODE);
  if (!node || !spa_streq(prop(node, OWNER), owner))
    return -1;
  for (int i = 0; i < 8; i++)
    if (inputs[i]->id != refs->ports[i].id || serial(inputs[i]) != refs->ports[i].serial)
      return -1;
  for (int i = 0; i < 2; i++)
    if (outputs[i]->id != refs->monitors[i].id || serial(outputs[i]) != refs->monitors[i].serial)
      return -1;
  unsigned owned = 0, outgoing = 0;
  for (struct created *c = s->created; c; c = c->next) {
    if (!c->proxy || c->removed || c->error || !c->bound || !c->info)
      return -1;
    owned++;
  }
  for (size_t i = 0; i < s->count; i++) {
    struct object *l = s->objects[i];
    if (!l->present || l->kind != LINK)
      continue;
    if (l->out_node == source->id || l->in_node == refs->node.id)
      outgoing++;
  }
  if (owned != 3 || outgoing != 2)
    return -1;
  for (int i = 0; i < 2; i++) {
    struct object *l = find(s, refs->links[i].id, LINK);
    if (!l || !l->info || serial(l) != refs->links[i].serial ||
        l->state != PW_LINK_STATE_ACTIVE || !spa_streq(prop(l, OWNER), owner) ||
        l->out_node != source->id || l->out_port != outputs[i]->id ||
        l->in_node != refs->node.id || l->in_port != inputs[(pair-1)*2+i]->id)
      return -1;
  }
  return 0;
}
#ifdef BCL_TEST_FAULTS
int bcl_playback_test_fault(struct session *s, int stage) {
  if (!s || s->catalog_only || s->created || s->pending ||
      stage < 1 || stage > 4 || s->playback_fault)
    return -1;
  s->playback_fault = stage;
  return 0;
}
#endif
