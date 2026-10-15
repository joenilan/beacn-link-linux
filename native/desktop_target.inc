/* Explicit desktop target lease. The session manager owns stream links.
 * No registry destroy, foreign proxy mutation, defaults or volume writes.
 * This entry cannot use the private executor or physical playback factory. */
#define DT_ENTRIES 4096
struct dt_entry {
  uint32_t subject;
  bool present;
  char type[33], value[129];
};
struct desktop_target {
  struct pw_metadata *metadata;
  struct spa_hook listener;
  struct receipt metadata_ref, sink, source, client, original;
  struct dt_entry entries[DT_ENTRIES], prior;
  size_t count;
  uint32_t pid;
  unsigned pair;
  char owner[129];
  struct created *switch_bridge[2];
  struct receipt source_ports[2];
  bool active, conflict, stopping, expected_present, source_removed, cache_guard;
  char expected_type[33], expected[129];
  struct created *bridge[2];
  struct receipt hardware;
  struct playback_refs adapter;
  struct created *adapter_proxy;
#ifdef BCL_TEST_FAULTS
  bool fail_after_target_echo;
#endif
};
int bcl_desktop_unbridge(struct session *);
int bcl_desktop_bridge_check(struct session *);
static struct desktop_target *desktop_new(void) {
  return calloc(1, sizeof(struct desktop_target));
}
static void desktop_removed(struct session *s, uint32_t id) {
  struct desktop_target *d = s->desktop;
  if (d->metadata && d->metadata_ref.id == id)
    s->fatal = true;
  /* global_remove is delivered before the generic object ledger marks the
   * record absent. Only the pinned old source generation establishes this
   * flag; a numerically reused node is never accepted as its replacement. */
  if (d->source.serial && d->source.id == id)
    for (size_t i = 0; i < s->count; i++) {
      struct object *o = s->objects[i];
      if (o->kind == NODE && o->id == d->source.id && o->present &&
          serial(o) == d->source.serial) {
        d->source_removed = true;
        break;
      }
    }
}
static struct dt_entry *dt_entry(struct desktop_target *d, uint32_t subject) {
  for (size_t i = 0; i < d->count; i++)
    if (d->entries[i].subject == subject)
      return &d->entries[i];
  return NULL;
}
static bool dt_subject_present(struct session *s, uint32_t subject) {
  for (size_t i = 0; i < s->count; i++)
    if (s->objects[i]->present && s->objects[i]->id == subject)
      return true;
  return false;
}
static bool dt_equal(struct dt_entry *e, bool present, const char *type, const char *value) {
  return (!e || !e->present) ? !present : present &&
      spa_streq(e->type, type) && spa_streq(e->value, value);
}
static int dt_property(void *data, uint32_t subject, const char *key,
                       const char *type, const char *value) {
  struct session *s = data;
  struct desktop_target *d = s->desktop;
  if (!key) {
    /* PipeWire can emit a subject-clear after the exact source generation was
     * removed. Ignore only that narrow stale-source clear, never a clear for a
     * live replacement or any other metadata subject. */
    if (d->active && subject == d->source.id && d->source_removed &&
        !dt_subject_present(s, subject))
      return 0;
    fprintf(stderr, "desktop metadata clear refused: source_subject=%d source_removed=%d subject_present=%d\n",
            subject == d->source.id, d->source_removed, dt_subject_present(s, subject));
    s->fatal = true;
    return 0;
  }
  if (subject == 0 && spa_streq(key, "linx.target-cache-guard")) {
    d->cache_guard = type && value && spa_streq(type, "Spa:String") && spa_streq(value, "1");
    return 0;
  }
  if (d->active && ((subject == 0 && strstr(key, "default.") == key) ||
       (subject == d->source.id && spa_streq(key, "target.node"))))
    d->conflict = true;
  if (!spa_streq(key, "target.object"))
    return 0;
  if ((value && (!type || strlen(type) > 32 || strlen(value) > 128))) {
    s->fatal = true; return 0;
  }
  struct dt_entry *e = dt_entry(d, subject);
  if (!e) {
    if (d->count == DT_ENTRIES) { s->fatal = true; return 0; }
    e = &d->entries[d->count++]; e->subject = subject;
  }
  e->present = value != NULL;
  snprintf(e->type, sizeof(e->type), "%s", value ? type : "");
  snprintf(e->value, sizeof(e->value), "%s", value ? value : "");
  if (d->active && subject == d->source.id &&
      !dt_equal(e, d->expected_present, d->expected_type, d->expected)) {
    fprintf(stderr, "desktop metadata conflict: present=%d expected=%d type_match=%d value_match=%d\n",
            e->present, d->expected_present, spa_streq(e->type, d->expected_type), spa_streq(e->value, d->expected));
    d->conflict = true;
  }
  return 0;
}
static const struct pw_metadata_events dt_events = {
    .version = PW_VERSION_METADATA_EVENTS, .property = dt_property};
static bool desktop_global(struct session *s, uint32_t id, uint32_t permissions,
                           const char *type, uint32_t version, const struct spa_dict *props) {
  if (spa_streq(type, PW_TYPE_INTERFACE_Device) || spa_streq(type, PW_TYPE_INTERFACE_Module))
    return true;
  if (!spa_streq(type, PW_TYPE_INTERFACE_Metadata))
    return false;
  if (!props || !spa_streq(spa_dict_lookup(props, PW_KEY_METADATA_NAME), "default"))
    return true;
  struct desktop_target *d = s->desktop;
  if (d->metadata || !(permissions & PW_PERM_R) || !(permissions & PW_PERM_W) ||
      !(permissions & PW_PERM_X) || !num(spa_dict_lookup(props, PW_KEY_OBJECT_SERIAL))) {
    s->fatal = true; return true;
  }
  d->metadata_ref = (struct receipt){id, num(spa_dict_lookup(props, PW_KEY_OBJECT_SERIAL))};
  d->metadata = pw_registry_bind(s->registry, id, type,
      version < PW_VERSION_METADATA ? version : PW_VERSION_METADATA, 0);
  if (!d->metadata || pw_metadata_add_listener(d->metadata, &d->listener, &dt_events, s) < 0)
    s->fatal = true;
  return true;
}
static void desktop_free(struct session *s) {
  struct desktop_target *d = s->desktop;
  if (d->metadata) {
    spa_hook_remove(&d->listener);
    pw_proxy_destroy((struct pw_proxy *)d->metadata);
  }
  free(d); s->desktop = NULL;
}
struct session *bcl_open_desktop_target(int fd, const char *config, uint32_t cookie,
                                       const char *name, uint32_t server_pid) {
  struct session *s = open_session(fd, config, cookie, name, server_pid, false, true);
  if (s && !s->desktop->metadata) { bcl_close(s); return NULL; }
  return s;
}
uint32_t bcl_desktop_daemon_pid(struct session *s) {
  return s && s->desktop && !s->fatal ? s->daemon_pid : 0;
}
static bool dt_ref(struct object *o, struct receipt r) {
  return o && o->info && o->present && serial(o) == r.serial && o->id == r.id;
}
static bool dt_receipt_present(struct session *s, struct receipt r, uint32_t kind) {
  for (size_t i = 0; i < s->count; i++) {
    struct object *o = s->objects[i];
    if (o->present && o->id == r.id && o->kind == kind && serial(o) == r.serial)
      return true;
  }
  return false;
}
static bool dt_source_gone_confirmed(struct session *s) {
  struct desktop_target *d = s ? s->desktop : NULL;
  if (!d || !d->source_removed || !d->source.serial ||
      dt_receipt_present(s, d->source, NODE))
    return false;
  for (int i = 0; i < 2; i++)
    if (!d->source_ports[i].serial ||
        dt_receipt_present(s, d->source_ports[i], PORT))
      return false;
  return true;
}
static int dt_pair_index(unsigned pair, unsigned side) {
  if (pair < 1 || pair > 4 || side > 1) return -1;
  return (int)((pair - 1) * 2 + side);
}
/* Retire only a confirmed, detached creation record. Caller must have removed
 * listeners, destroyed the proxy, synchronized, and checked the old serial is
 * absent. Match the record pointer so a reused numeric ID cannot select another
 * creation, and preserve any record that may still be pending/current. */
static bool dt_retire_created(struct session *s, struct created *c,
                              struct receipt old) {
  if (!s || !s->desktop || !c || s->pending || c->proxy ||
      c->id != old.id || c->serial != old.serial)
    return false;
  if (s->desktop->adapter_proxy == c) return false;
  for (int i=0;i<2;i++)
    if (s->desktop->bridge[i] == c || s->desktop->switch_bridge[i] == c) return false;
  struct object *o=find(s,old.id,LINK);
  if (o && serial(o)==old.serial) return false;
  struct created **slot=&s->created;
  while (*slot && *slot != c) slot=&(*slot)->next;
  if (*slot != c) return false;
  *slot=c->next;
  c->next=NULL;
  free(c);
  return true;
}
static bool dt_ports_mode(struct session *s, struct receipt node, int direction, struct object **result, bool monitor) {
  result[0] = result[1] = NULL;
  for (size_t i = 0; i < s->count; i++) {
    struct object *p = s->objects[i];
    if (!p->present || p->kind != PORT || num(prop(p, PW_KEY_NODE_ID)) != node.id ||
        p->direction != direction)
      continue;
    const char *ch = prop(p, PW_KEY_AUDIO_CHANNEL);
    int side = spa_streq(ch, "FL") ? 0 : spa_streq(ch, "FR") ? 1 : -1;
    if (!p->info || !serial(p) || side < 0 || result[side] ||
        spa_streq(prop(p, PW_KEY_PORT_MONITOR), "true") != monitor ||
        spa_streq(prop(p, PW_KEY_PORT_CONTROL), "true"))
      return false;
    result[side] = p;
  }
  return result[0] && result[1];
}
static bool dt_ports(struct session *s, struct receipt node, int direction, struct object **result) {
  return dt_ports_mode(s,node,direction,result,false);
}
static bool dt_source(struct session *s) {
  struct desktop_target *d = s->desktop;
  struct object *n = find(s, d->source.id, NODE), *c = find(s, d->client.id, CLIENT);
  if (!dt_ref(n, d->source) || !dt_ref(c, d->client) ||
      !spa_streq(prop(n, PW_KEY_MEDIA_CLASS), "Stream/Output/Audio") ||
      num(prop(n, PW_KEY_CLIENT_ID)) != d->client.id ||
      num(prop(c, PW_KEY_APP_PROCESS_ID)) != d->pid ||
      spa_streq(prop(n, "node.dont-move"), "true"))
    return false;
  struct object *ports[2];
  if (!dt_ports(s, d->source, PW_DIRECTION_OUTPUT, ports))
    return false;
  for (int i = 0; i < 2; i++)
    if (d->source_ports[i].serial && !dt_ref(ports[i], d->source_ports[i]))
      return false;
  return true;
}
static bool dt_link_state_allowed(int node_state, int link_state,
                                  bool cleanup_idle) {
  if (link_state == PW_LINK_STATE_ACTIVE) return true;
  /* WirePlumber reconnects an idle stream with INIT links until it wakes.
   * This is restored target intent only, never a live playback proof. */
  return cleanup_idle &&
      (link_state == PW_LINK_STATE_PAUSED || link_state == PW_LINK_STATE_INIT) &&
      (node_state == PW_NODE_STATE_IDLE || node_state == PW_NODE_STATE_SUSPENDED);
}
static bool dt_route_mode(struct session *s, struct receipt sink,
                          bool cleanup_idle) {
  struct desktop_target *d = s->desktop;
  struct object *source = find(s, d->source.id, NODE);
  struct object *n = find(s, sink.id, NODE), *out[2], *in[2];
  if (!dt_source(s) || !source || !dt_ref(n, sink) ||
      !spa_streq(prop(n, PW_KEY_MEDIA_CLASS), "Audio/Sink") ||
      !dt_ports(s, d->source, PW_DIRECTION_OUTPUT, out) ||
      !dt_ports(s, sink, PW_DIRECTION_INPUT, in))
    return false;
  unsigned counts[2] = {0, 0};
  for (size_t i = 0; i < s->count; i++) {
    struct object *l = s->objects[i];
    if (!l->present || l->kind != LINK || l->out_node != d->source.id)
      continue;
    int side = l->out_port == out[0]->id ? 0 : l->out_port == out[1]->id ? 1 : -1;
    if (!l->info || !serial(l) || side < 0 || l->in_node != sink.id ||
        l->in_port != in[side]->id ||
        !dt_link_state_allowed(source->state, l->state, cleanup_idle))
      return false;
    counts[side]++;
  }
  return counts[0] == 1 && counts[1] == 1;
}
static bool dt_route(struct session *s, struct receipt sink) {
  return dt_route_mode(s, sink, false);
}
static int dt_wait_mode(struct session *s, struct receipt sink,
                        bool cleanup_idle) {
  int64_t deadline = millis() + 5000;
  do {
    if (barrier(s) < 0 || barrier(s) < 0 || s->desktop->conflict || !dt_source(s)) {
      fprintf(stderr, "desktop route wait refused: fatal=%d conflict=%d source=%d\n", s->fatal, s->desktop->conflict, dt_source(s));
      return -1;
    }
    if (dt_route_mode(s, sink, cleanup_idle))
      return 0;
    if (pw_loop_iterate(pw_main_loop_get_loop(s->loop), 20) < 0)
      return -1;
  } while (millis() < deadline);
  return -1;
}
static int dt_wait(struct session *s, struct receipt sink) {
  return dt_wait_mode(s, sink, false);
}
static int dt_wait_cleanup(struct session *s, struct receipt sink) {
  return dt_wait_mode(s, sink, true);
}
static int dt_set(struct session *s, bool present, const char *type, const char *value) {
  struct desktop_target *d = s->desktop;
  if (barrier(s) < 0 || d->conflict ||
      !dt_equal(dt_entry(d, d->source.id), d->expected_present, d->expected_type, d->expected))
    return -1;
  d->expected_present = present;
  snprintf(d->expected_type, sizeof(d->expected_type), "%s", present ? type : "");
  snprintf(d->expected, sizeof(d->expected), "%s", present ? value : "");
  if (pw_metadata_set_property(d->metadata, d->source.id, "target.object",
      present ? type : NULL, present ? value : NULL) < 0)
    return -1;
  /* default metadata is implemented by another client. Core sync acknowledges
   * our request, not that client's eventual property event. Submit only once
   * and await the exact echo; unrelated values latch a conflict above. */
  int64_t deadline = millis() + 2000;
  do {
    if (barrier(s) < 0 || d->conflict) return -1;
    if (dt_equal(dt_entry(d, d->source.id), present, type, value)) return 0;
    if (pw_loop_iterate(pw_main_loop_get_loop(s->loop), 20) < 0) return -1;
  } while (millis() < deadline);
  fprintf(stderr, "desktop metadata echo deadline exceeded\n");
  return -1;
}
int bcl_desktop_cache_guard(struct session *s) {
  if (!s || !s->desktop || barrier(s) < 0 || barrier(s) < 0) return -1;
  return s->desktop->cache_guard ? 1 : 0;
}
int bcl_desktop_sink(struct session *s, const char *owner, struct receipt *result) {
  if (!s || !s->desktop || s->desktop->sink.serial || s->created || !label(owner) ||
      barrier(s) < 0)
    return -1;
  if (!s->desktop->cache_guard) return -2;
  struct pw_properties *p = pw_properties_new("factory.name", "support.null-audio-sink",
      PW_KEY_NODE_NAME, owner, PW_KEY_NODE_DESCRIPTION, "LinX desktop routing check",
      PW_KEY_MEDIA_CLASS, "Audio/Sink", PW_KEY_OBJECT_LINGER, "false",
      PW_KEY_NODE_AUTOCONNECT, "false", "node.virtual", "true", "node.driver", "true",
      "linx.temporary-target", "1",
      "state.restore-props", "false", "state.restore-target", "false",
      "priority.session", "0", "audio.rate", "48000", "audio.channels", "2",
      "audio.position", "[ FL FR ]", OWNER, owner,
      "adapter.auto-port-config", "{ mode = dsp monitor = true position = preserve }", NULL);
  if (!p) return -1;
  struct created *c = playback_created(s, "adapter", PW_TYPE_INTERFACE_Node,
                                      PW_VERSION_NODE, p, owner, true, NULL, NULL);
  pw_properties_free(p);
  if (!c || playback_ack(s, c, true, result) < 0) return -1;
  s->desktop->sink = *result;
  int64_t deadline = millis() + 5000;
  struct object *ports[2];
  do {
    if (barrier(s) < 0 || barrier(s) < 0) return -1;
    if (dt_ports(s, *result, PW_DIRECTION_INPUT, ports)) return 0;
    if (pw_loop_iterate(pw_main_loop_get_loop(s->loop), 20) < 0) return -1;
  } while (millis() < deadline);
  return -1;
}
int bcl_desktop_move(struct session *s, const struct receipt *source,
                      uint64_t client_serial, uint32_t pid,
                      const struct receipt *original) {
  if (!s || !s->desktop || !source || !client_serial || !original || !pid ||
      !s->desktop->sink.serial || s->desktop->active || s->desktop->stopping ||
      barrier(s) < 0 || barrier(s) < 0)
    return -1;
  if (!s->desktop->cache_guard) return -1;
  struct desktop_target *d = s->desktop;
  struct object *node = find(s, source->id, NODE);
  uint64_t cid = node ? num(prop(node, PW_KEY_CLIENT_ID)) : 0;
  if (!cid || cid > UINT32_MAX) return -1;
  d->source = *source; d->client = (struct receipt){(uint32_t)cid, client_serial};
  d->pid = pid; d->original = *original;
  if (!dt_route(s, *original)) return -1;
  struct object *ports[2];
  if (!dt_ports(s, *source, PW_DIRECTION_OUTPUT, ports)) return -1;
  for (int i = 0; i < 2; i++)
    d->source_ports[i] = (struct receipt){ports[i]->id, serial(ports[i])};
  struct dt_entry *prior = dt_entry(d, source->id);
  if (prior) d->prior = *prior;
  d->expected_present = d->prior.present;
  snprintf(d->expected_type, sizeof(d->expected_type), "%s", d->prior.type);
  snprintf(d->expected, sizeof(d->expected), "%s", d->prior.value);
  d->active = true;
  char target[32]; snprintf(target, sizeof(target), "%" PRIu64, d->sink.serial);
  if (dt_set(s, true, "Spa:Id", target) < 0)
    return -1;
#ifdef BCL_TEST_FAULTS
  if (d->fail_after_target_echo) {
    d->fail_after_target_echo=false;
    return -1;
  }
#endif
  if (dt_wait(s, d->sink) < 0) return -1;
  return 0;
}
int bcl_desktop_check(struct session *s) {
  if (!s || !s->desktop || !s->desktop->active || barrier(s) < 0 || barrier(s) < 0 ||
      !s->desktop->cache_guard || s->desktop->conflict ||
      !dt_equal(dt_entry(s->desktop, s->desktop->source.id), true,
                 s->desktop->expected_type, s->desktop->expected) ||
      !dt_route(s, s->desktop->sink))
    return -1;
  return 0;
}
int bcl_desktop_stop(struct session *s) {
  if (!s || !s->desktop || !s->desktop->active) return 0;
  struct desktop_target *d = s->desktop;
  if (d->stopping) return -1;
  d->stopping = true;
  int unbridged=bcl_desktop_unbridge(s);
  if (unbridged < 0 || s->fatal || d->conflict ||
      barrier(s) < 0 || barrier(s) < 0) {
    fprintf(stderr,"desktop stop preflight refused: unbridge=%d fatal=%d conflict=%d source_removed=%d source_gone=%d\n",
            unbridged,s->fatal,d->conflict,d->source_removed,dt_source_gone_confirmed(s));
    return -1;
  }
  if (s->fatal || d->conflict ||
      !dt_ref(find(s, d->original.id, NODE), d->original))
    return -1;
  /* A closed selected stream has no subject on which its previous target can
   * be restored. The exact old node and both old ports must be gone after the
   * unbridge barrier. Do not write metadata to a reused numeric subject. */
  if (dt_source_gone_confirmed(s)) {
    d->active = false;
    return 1;
  }
  if (!dt_source(s)) return -1;
  char target[32]; snprintf(target, sizeof(target), "%" PRIu64, d->original.serial);
  if (dt_set(s, true, "Spa:Id", target) < 0 || dt_wait_cleanup(s, d->original) < 0 ||
      dt_set(s, d->prior.present, d->prior.type, d->prior.value) < 0 ||
      dt_wait_cleanup(s, d->original) < 0)
    return -1;
  d->active = false;
  return 0;
}
/* 1 means the pinned source generation and both pinned output ports are
 * confirmed absent; 0 means it is still present or removal is incomplete. */
int bcl_desktop_source_gone(struct session *s) {
  if (!s || !s->desktop || !s->desktop->active || s->fatal ||
      barrier(s) < 0 || barrier(s) < 0 || s->fatal)
    return -1;
  return dt_source_gone_confirmed(s) ? 1 : 0;
}
int bcl_desktop_release_sink(struct session *s) {
  if (!s || !s->desktop || s->desktop->active || !s->desktop->sink.serial)
    return -1;
  struct created *c = s->created;
  while (c && (c->id != s->desktop->sink.id || c->serial != s->desktop->sink.serial)) c=c->next;
  if (!c || !c->bound || c->id != s->desktop->sink.id ||
      c->serial != s->desktop->sink.serial || !c->proxy)
    return -1;
  spa_hook_remove(&c->listener); spa_hook_remove(&c->info_listener);
  pw_proxy_destroy(c->proxy); c->proxy = NULL;
  if (barrier(s) < 0 || barrier(s) < 0 || find(s, c->id, NODE))
    return -1;
  return 0;
}
static bool dt_bridge_graph(struct session *s) {
  struct desktop_target *d=s->desktop;
  if (d->switch_bridge[0] || d->switch_bridge[1]) return false;
  struct object *hardware=find(s,d->hardware.id,NODE), *inputs[8];
  struct object *adapter=find(s,d->adapter.node.id,NODE);
  if (d->pair < 1 || d->pair > 4 || !dt_ref(hardware,d->hardware) || !dt_route(s,d->sink) ||
      !dt_ref(adapter,d->adapter.node) || !d->adapter_proxy ||
      !d->adapter_proxy->proxy || d->adapter_proxy->removed || d->adapter_proxy->error ||
      !playback_ports(s,&d->adapter.node,inputs)) return false;
  for (int i=0;i<8;i++)
    if (!dt_ref(inputs[i],d->adapter.ports[i])) return false;
  unsigned count=0;
  for (size_t i=0;i<s->count;i++) {
    struct object *l=s->objects[i];
    if (!l->present || l->kind!=LINK) continue;
    if (l->in_node==d->hardware.id) return false;
    if (l->in_node!=d->adapter.node.id) continue;
    struct created *c=NULL;
    for (int j=0;j<2;j++)
      if (d->bridge[j] && d->bridge[j]->id==l->id && d->bridge[j]->serial==serial(l)) c=d->bridge[j];
    if (!c || !c->proxy || c->removed || c->error || !l->info || l->state!=PW_LINK_STATE_ACTIVE ||
        l->out_node!=d->sink.id || l->out_port!=c->out_port || l->in_port!=c->in_port)
      return false;
    bool expected_pair_port=false;
    for (int j=0;j<2;j++)
      if (d->bridge[j]==c && l->in_port==d->adapter.ports[dt_pair_index(d->pair,(unsigned)j)].id)
        expected_pair_port=true;
    if (!expected_pair_port) return false;
    count++;
  }
  return count==2;
}
/* Read-only discovery on the already authenticated desktop connection. This is
 * a candidate identity, never removal authority or permission to skip bridge checks. */
int bcl_desktop_find_studio(struct session *s, uint32_t card, struct receipt *result) {
  if (!s || !s->desktop || !result || card>255 || barrier(s)<0) return -1;
  char path[32],path0[32];snprintf(path,sizeof(path),"hw:%u",card);snprintf(path0,sizeof(path0),"hw:%u,0",card);
  struct object *selected=NULL;
  for (size_t i=0;i<s->count;i++) {
    struct object *n=s->objects[i];
    if (!n->present || n->kind!=NODE || !n->info || !serial(n) ||
        !spa_streq(prop(n,PW_KEY_MEDIA_CLASS),"Audio/Sink") ||
        !(spa_streq(prop(n,"api.alsa.path"),path) || spa_streq(prop(n,"api.alsa.path"),path0)) ||
        num(prop(n,"audio.channels"))!=8) continue;
    if (selected) return -1;
    selected=n;
  }
  if (!selected) return -1;
  result->id=selected->id;result->serial=serial(selected);return 0;
}
int bcl_desktop_bridge(struct session *s, const struct receipt *hardware,
                        uint32_t card, int pair, const char *owner, struct receipt *links,
                        struct receipt *adapter_result) {
  if (!s || !s->desktop || !hardware || !links || !label(owner) || card>255 || pair<1 || pair>4 ||
      s->desktop->hardware.serial || bcl_desktop_check(s)<0) return -1;
  struct desktop_target *d=s->desktop;
  struct object *node=find(s,hardware->id,NODE);
  char path[32],path0[32];snprintf(path,sizeof(path),"hw:%u",card);snprintf(path0,sizeof(path0),"hw:%u,0",card);
  if (!dt_ref(node,*hardware) || !spa_streq(prop(node,PW_KEY_MEDIA_CLASS),"Audio/Sink") ||
      !(spa_streq(prop(node,"api.alsa.path"),path) || spa_streq(prop(node,"api.alsa.path"),path0))) return -1;
  for (size_t i=0;i<s->count;i++) {
    struct object *l=s->objects[i];
    if (l->present && l->kind==LINK && l->in_node==node->id) return -1;
  }
  struct object *out[2]={NULL,NULL};unsigned input_count=0;
  const char *channels[]={"FL","FR","RL","RR","FC","LFE","SL","SR"};
  bool seen[8]={false};
  for (size_t i=0;i<s->count;i++) {
    struct object *p=s->objects[i];
    if (!p->present || p->kind!=PORT) continue;
    uint64_t parent=num(prop(p,PW_KEY_NODE_ID));const char *ch=prop(p,PW_KEY_AUDIO_CHANNEL);
    if (parent==d->sink.id && p->direction==PW_DIRECTION_OUTPUT) {
      int side=spa_streq(ch,"FL")?0:spa_streq(ch,"FR")?1:-1;
      if (!p->info || !serial(p) || side<0 || out[side] || !spa_streq(prop(p,PW_KEY_PORT_MONITOR),"true")) return -1;
      out[side]=p;
    }
    if (parent==hardware->id && p->direction==PW_DIRECTION_INPUT) {
      int side=-1;for (int j=0;j<8;j++) if (spa_streq(ch,channels[j])) side=j;
      if (!p->info || !serial(p) || side<0 || seen[side]) return -1;
      seen[side]=true;input_count++;
    }
  }
  if (input_count!=8 || !out[0] || !out[1] || !adapter_result) return -1;
  d->hardware=*hardware;
  d->pair=(unsigned)pair;
  snprintf(d->owner,sizeof(d->owner),"%s",owner);
  struct object *inputs[8];
  int created=playback_adapter_create(s,d->sink.id,owner,card,0,true,&d->adapter,inputs);
  for (struct created *c=s->created;c;c=c->next)
    if (c->bound && c->id==d->adapter.node.id && c->serial==d->adapter.node.serial)
      d->adapter_proxy=c;
  if (created<0 || !d->adapter_proxy) return -1;
  *adapter_result=d->adapter.node;
  for (int i=0;i<2;i++) {
    struct pw_properties *p=pw_properties_new(PW_KEY_OBJECT_LINGER,"false",PW_KEY_LINK_PASSIVE,"false",OWNER,owner,NULL);
    if (!p) return -1;
    d->bridge[i]=playback_created(s,"link-factory",PW_TYPE_INTERFACE_Link,PW_VERSION_LINK,p,owner,false,out[i],inputs[dt_pair_index(d->pair,(unsigned)i)]);
    pw_properties_free(p);
    if (!d->bridge[i] || playback_ack(s,d->bridge[i],false,&links[i])<0) return -1;
  }
  return dt_bridge_graph(s)?0:-1;
}
int bcl_desktop_switch_pair(struct session *s, int pair, struct receipt *links) {
  if (!s || !s->desktop || !links || pair<1 || pair>4 || s->pending ||
      bcl_desktop_bridge_check(s)<0)
    return -1;
  struct desktop_target *d=s->desktop;
  if ((unsigned)pair==d->pair) {
    for (int i=0;i<2;i++) links[i]=(struct receipt){d->bridge[i]->id,d->bridge[i]->serial};
    return 0;
  }

  /* Keep the old pair active until both replacement links are acknowledged.
   * Otherwise the retained ALSA driver pauses/re-prepares with no active input,
   * resetting the hardware clock and correctly invalidating both PCM proofs.
   * Every partial replacement is retained for the ordinary stop owner. */
  struct object *out[2]={NULL,NULL}, *inputs[8];
  if (!dt_ports_mode(s,d->sink,PW_DIRECTION_OUTPUT,out,true) ||
      !playback_ports(s,&d->adapter.node,inputs)) return -6;
  for (int i=0;i<8;i++)
    if (!dt_ref(inputs[i],d->adapter.ports[i])) return -7;
  for (int i=0;i<2;i++) {
    struct pw_properties *p=pw_properties_new(PW_KEY_OBJECT_LINGER,"false",PW_KEY_LINK_PASSIVE,"false",OWNER,d->owner,NULL);
    if (!p) return -8;
    struct created *c=playback_created(s,"link-factory",PW_TYPE_INTERFACE_Link,
        PW_VERSION_LINK,p,d->owner,false,out[i],inputs[dt_pair_index((unsigned)pair,(unsigned)i)]);
    pw_properties_free(p);d->switch_bridge[i]=c;
    if (!c || playback_ack(s,c,false,&links[i])<0) return -9;
  }
  /* Retire only exact old proxies after the replacement is carrying audio. */
  struct receipt old[2];
  struct created *old_created[2];
  for (int i=0;i<2;i++) {
    struct created *c=d->bridge[i];
    if (!c || !c->proxy || c->removed || c->error || !c->bound || !c->info)
      return -2;
    old[i]=(struct receipt){c->id,c->serial};
    old_created[i]=c;
    spa_hook_remove(&c->listener);spa_hook_remove(&c->info_listener);
    pw_proxy_destroy(c->proxy);c->proxy=NULL;
  }
  if (barrier(s)<0 || barrier(s)<0) return -3;
  for (int i=0;i<2;i++) {
    struct object *o=find(s,old[i].id,LINK);
    if (o && serial(o)==old[i].serial) return -4;
    d->bridge[i]=d->switch_bridge[i];d->switch_bridge[i]=NULL;
  }
  for (int i=0;i<2;i++)
    if (!dt_retire_created(s,old_created[i],old[i])) return -5;

  d->pair=(unsigned)pair;
  return dt_bridge_graph(s)?0:-10;
}
int bcl_desktop_bridge_check(struct session *s) {
  return bcl_desktop_check(s)==0 && dt_bridge_graph(s)?0:-1;
}
int bcl_desktop_adapter_ready(struct session *s) {
  if (bcl_desktop_bridge_check(s)<0) return -1;
  struct created *c=s->desktop->adapter_proxy;
  struct object *n=find(s,s->desktop->adapter.node.id,NODE);
  if (!c || !n || !c->info || !n->info) return -1;
  if (c->state==PW_NODE_STATE_RUNNING && n->state==PW_NODE_STATE_RUNNING) return 1;
  if ((c->state==PW_NODE_STATE_IDLE || c->state==PW_NODE_STATE_SUSPENDED) &&
      (n->state==PW_NODE_STATE_IDLE || n->state==PW_NODE_STATE_SUSPENDED)) return 0;
  fprintf(stderr,"desktop adapter readiness refused: retained=%d observed=%d\n",c->state,n->state);
  return -1;
}
int bcl_desktop_unbridge(struct session *s) {
  if (!s || !s->desktop) return -1;
  struct desktop_target *d=s->desktop;
  for (int i=0;i<2;i++) {
    struct created *pending=d->switch_bridge[i];
    if (pending && pending->proxy) {
      spa_hook_remove(&pending->listener);spa_hook_remove(&pending->info_listener);
      pw_proxy_destroy(pending->proxy);pending->proxy=NULL;
    }
    struct created *c=d->bridge[i];
    if (!c || !c->proxy) continue;
    spa_hook_remove(&c->listener);spa_hook_remove(&c->info_listener);
    pw_proxy_destroy(c->proxy);c->proxy=NULL;
  }
  if (d->adapter_proxy && d->adapter_proxy->proxy) {
    struct created *c=d->adapter_proxy;
    spa_hook_remove(&c->listener);spa_hook_remove(&c->info_listener);
    pw_proxy_destroy(c->proxy);c->proxy=NULL;
  }
  if (!d->hardware.serial) return 0;
  if (barrier(s)<0 || barrier(s)<0) return -1;
  for (int i=0;i<2;i++) {
    struct created *pending=d->switch_bridge[i];
    struct object *fresh=pending?find(s,pending->id,LINK):NULL;
    if (fresh && serial(fresh)==pending->serial) return -1;
    struct created *c=d->bridge[i];
    struct object *o=c?find(s,c->id,LINK):NULL;
    if (o && serial(o)==c->serial) return -1;
  }
  if (dt_ref(find(s,d->adapter.node.id,NODE),d->adapter.node)) return -1;
  return 0;
}
#ifdef BCL_TEST_FAULTS
int bcl_desktop_test_fail_after_target_echo(struct session *s) {
  if (!s || !s->desktop || s->desktop->active || s->created) return -1;
  s->desktop->fail_after_target_echo=true;
  return 0;
}
int bcl_desktop_test_core_identity(void) {
  struct desktop_target desktop={0};
  struct session s={.desktop=&desktop,.cookie=7};
  snprintf(s.name,sizeof(s.name),"test-core");
  struct spa_dict_item item={PW_KEY_APP_PROCESS_ID,"41"};
  struct spa_dict props=SPA_DICT_INIT(&item,1);
  struct pw_core_info info={.cookie=7,.name="test-core",.props=&props};
  core_info(&s,&info);
  if (s.fatal || s.daemon_pid!=41 || !s.core_seen) return -1;
  /* Same daemon is stable even when the authenticated socket peer differs. */
  core_info(&s,&info);
  if (s.fatal || s.daemon_pid!=41) return -1;
  item.value="42";core_info(&s,&info);
  if (!s.fatal) return -1;
  for (int mode=0;mode<4;mode++) {
    memset(&s,0,sizeof(s));s.desktop=&desktop;s.cookie=7;
    snprintf(s.name,sizeof(s.name),"test-core");
    const char *invalid[]={"0","4294967296","garbage",""};item.value=invalid[mode];
    core_info(&s,&info);if (!s.fatal) return -1;
  }
  return 7;
}
static int bcl_desktop_test_ledger_retirement(void) {
  struct desktop_target desktop={0};
  struct session s={.desktop=&desktop};
  struct created before={.id=30,.serial=999}, after={.id=31,.serial=131};
  struct object observed={0};
  struct created *target=calloc(1,sizeof(*target));
  if (!target) return -1;
  target->id=30;target->serial=110;target->next=&after;
  before.next=target;s.created=&before;
  struct receipt old={30,110};
  int checks=0;

  s.pending=target;
  if (dt_retire_created(&s,target,old)) { target=NULL;goto fail; }
  if (s.created!=&before || before.next!=target) goto fail;
  checks++;s.pending=NULL;

  target->proxy=(struct pw_proxy *)target;
  if (dt_retire_created(&s,target,old)) { target=NULL;goto fail; }
  if (s.created!=&before || before.next!=target) goto fail;
  checks++;target->proxy=NULL;

  desktop.bridge[0]=target;
  if (dt_retire_created(&s,target,old)) { target=NULL;goto fail; }
  if (s.created!=&before || before.next!=target) goto fail;
  checks++;desktop.bridge[0]=NULL;

  observed.id=30;observed.kind=LINK;observed.present=true;
  observed.props=pw_properties_new(PW_KEY_OBJECT_SERIAL,"110",NULL);
  if (!observed.props) goto fail;
  s.objects[0]=&observed;s.count=1;
  if (dt_retire_created(&s,target,old)) {
    target=NULL;
    pw_properties_free(observed.props);goto fail;
  }
  if (s.created!=&before || before.next!=target) {
    pw_properties_free(observed.props);goto fail;
  }
  checks++;
  pw_properties_set(observed.props,PW_KEY_OBJECT_SERIAL,"111");
  if (!dt_retire_created(&s,target,old)) {
    pw_properties_free(observed.props);goto fail;
  }
  target=NULL;
  if (s.created!=&before || before.next!=&after || after.next ||
      before.id!=old.id || before.serial==old.serial) {
    pw_properties_free(observed.props);goto fail_freed;
  }
  checks++;
  pw_properties_free(observed.props);
  return checks;

fail:
  free(target);
  return -1;
fail_freed:
  return -1;
}
static int bcl_desktop_test_source_gone(void) {
  struct desktop_target desktop={0};
  struct session s={.desktop=&desktop};
  struct object objects[5]={0};
  desktop.source=(struct receipt){7,70};
  desktop.source_ports[0]=(struct receipt){8,80};
  desktop.source_ports[1]=(struct receipt){9,90};
  desktop.source_removed=true;
  const uint32_t ids[]={7,8,9,7,8};
  const uint64_t serials[]={70,80,90,71,81};
  const int kinds[]={NODE,PORT,PORT,NODE,PORT};
  int checks=0;
  for (int i=0;i<5;i++) {
    char serial_text[16];
    snprintf(serial_text,sizeof(serial_text),"%" PRIu64,serials[i]);
    objects[i].id=ids[i]; objects[i].kind=kinds[i]; objects[i].present=true;
    objects[i].props=pw_properties_new(PW_KEY_OBJECT_SERIAL,serial_text,NULL);
    if (!objects[i].props) goto fail;
    s.objects[s.count++]=&objects[i];
  }
  /* A retained old node is not gone, even when its ports are absent. */
  objects[1].present=false; objects[2].present=false;
  if (dt_source_gone_confirmed(&s)) goto fail;
  checks++;
  /* Old node disappearance alone is insufficient while either old port lives. */
  objects[0].present=false; objects[1].present=true;
  if (dt_source_gone_confirmed(&s)) goto fail;
  checks++;
  objects[1].present=false;
  if (!dt_source_gone_confirmed(&s)) goto fail;
  checks++;
  desktop.source_removed=false;
  if (dt_source_gone_confirmed(&s)) goto fail;
  desktop.source_removed=true;
  checks++;
  /* Numeric-ID reuse is not the pinned old generation, but its subject clear
   * must remain fatal and therefore cannot be mistaken for stale cleanup. */
  desktop.active=true;
  dt_property(&s,7,NULL,NULL,NULL);
  if (!s.fatal) goto fail;
  checks++;
  s.fatal=false;
  objects[3].present=false;
  /* Once the exact source and ports are gone and no replacement subject is
   * present, the corresponding stale subject-clear is harmless. */
  dt_property(&s,7,NULL,NULL,NULL);
  if (s.fatal) goto fail;
  checks++;
  /* The exemption never applies to another subject. */
  dt_property(&s,0,NULL,NULL,NULL);
  if (!s.fatal) goto fail;
  checks++;
  for (int i=0;i<5;i++) pw_properties_free(objects[i].props);
  return checks;
fail:
  for (int i=0;i<5;i++) if (objects[i].props) pw_properties_free(objects[i].props);
  return -1;
}
static int bcl_desktop_test_cleanup_link_states(void) {
  int checks=0;
  if (!dt_link_state_allowed(PW_NODE_STATE_RUNNING, PW_LINK_STATE_ACTIVE, false) ||
      !dt_link_state_allowed(PW_NODE_STATE_RUNNING, PW_LINK_STATE_ACTIVE, true))
    return -1;
  checks++;
  if (!dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_ACTIVE, false))
    return -1;
  checks++;
  if (dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_PAUSED, false) ||
      !dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_PAUSED, true))
    return -1;
  checks++;
  if (!dt_link_state_allowed(PW_NODE_STATE_SUSPENDED, PW_LINK_STATE_PAUSED, true))
    return -1;
  checks++;
  if (dt_link_state_allowed(PW_NODE_STATE_RUNNING, PW_LINK_STATE_PAUSED, true))
    return -1;
  checks++;
  if (dt_link_state_allowed(PW_NODE_STATE_ERROR, PW_LINK_STATE_PAUSED, true))
    return -1;
  checks++;
  if (dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_NEGOTIATING, true))
    return -1;
  checks++;
  if (dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_ERROR, true))
    return -1;
  checks++;
  if (!dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_INIT, true) ||
      dt_link_state_allowed(PW_NODE_STATE_RUNNING, PW_LINK_STATE_INIT, true) ||
      dt_link_state_allowed(PW_NODE_STATE_IDLE, PW_LINK_STATE_INIT, false))
    return -1;
  checks++;
  return checks;
}
static int bcl_desktop_test_cache_guard(void) {
  struct session s={0}; s.desktop=desktop_new();
  if (!s.desktop) return -1;
  struct desktop_target *d=s.desktop;
  bool ok=!d->cache_guard;
  dt_property(&s,0,"linx.target-cache-guard","Spa:String","1");
  ok=ok && d->cache_guard && !d->conflict && !s.fatal;
  dt_property(&s,7,"linx.target-cache-guard","Spa:String","0");
  ok=ok && d->cache_guard;
  dt_property(&s,0,"linx.target-cache-guard","Spa:String","2");
  ok=ok && !d->cache_guard;
  dt_property(&s,0,"linx.target-cache-guard","Spa:Id","1");
  ok=ok && !d->cache_guard;
  dt_property(&s,0,"linx.target-cache-guard","Spa:String","1");
  dt_property(&s,0,"linx.target-cache-guard",NULL,NULL);
  ok=ok && !d->cache_guard && !d->conflict && !s.fatal;
  free(d);
  return ok ? 0 : -1;
}
static bool dt_test_port_fixture(const char **channels, const char **monitors,
                                const char **controls, size_t count, bool monitor) {
  if (count > 4) return false;
  struct session s = {0};
  struct object objects[4] = {0};
  struct object *result[2] = {NULL, NULL};
  bool valid = false;
  for (size_t i = 0; i < count; i++) {
    char id[16], serial_text[16];
    snprintf(id, sizeof(id), "%u", 7u);
    snprintf(serial_text, sizeof(serial_text), "%u", (unsigned)(20 + i));
    objects[i].id = (uint32_t)(30 + i);
    objects[i].kind = PORT;
    objects[i].present = true;
    objects[i].info = true;
    objects[i].direction = PW_DIRECTION_OUTPUT;
    objects[i].props = pw_properties_new(
        PW_KEY_NODE_ID, id,
        PW_KEY_OBJECT_SERIAL, serial_text,
        PW_KEY_AUDIO_CHANNEL, channels[i],
        NULL);
    if (!objects[i].props) goto done;
    if (monitors[i])
      pw_properties_set(objects[i].props, PW_KEY_PORT_MONITOR, monitors[i]);
    if (controls[i])
      pw_properties_set(objects[i].props, PW_KEY_PORT_CONTROL, controls[i]);
    s.objects[s.count++] = &objects[i];
  }
  valid = dt_ports_mode(&s, (struct receipt){.id = 7}, PW_DIRECTION_OUTPUT,
                        result, monitor);
done:
  for (size_t i = 0; i < count; i++)
    if (objects[i].props) pw_properties_free(objects[i].props);
  return valid;
}
static int bcl_desktop_test_port_monitor_modes(void) {
  const char *channels[] = {"FL", "FR", "FL", "MONO"};
  const char *unexpected[] = {"FL", "FR", "MONO", "FL"};
  const char *monitor[] = {"true", "true", "true", "true"};
  const char *ordinary[] = {NULL, NULL, NULL, NULL};
  const char *controls[] = {NULL, NULL, NULL, NULL};
  int checks = 0;

  /* The output of LinX's Studio sink is a monitor pair. */
  if (!dt_test_port_fixture(channels, monitor, controls, 2, true) ||
      dt_test_port_fixture(channels, monitor, controls, 2, false))
    return -1;
  checks += 2;
  /* An application's ordinary output pair is accepted only in normal mode. */
  if (!dt_test_port_fixture(channels, ordinary, controls, 2, false) ||
      dt_test_port_fixture(channels, ordinary, controls, 2, true))
    return -1;
  checks += 2;
  /* Duplicate/unexpected channels and control ports are refused in both modes. */
  if (dt_test_port_fixture(channels, ordinary, controls, 3, false) ||
      dt_test_port_fixture(unexpected, ordinary, controls, 3, false))
    return -1;
  checks += 2;
  controls[1] = "true";
  if (dt_test_port_fixture(channels, monitor, controls, 2, true))
    return -1;
  checks++;
  return checks;
}
int bcl_desktop_test_pair_mapping(void) {
  if (bcl_desktop_test_cache_guard()!=0) return -1;
  if (bcl_desktop_test_ledger_retirement()!=5) return -1;
  if (bcl_desktop_test_source_gone()!=7) return -1;
  if (bcl_desktop_test_cleanup_link_states()!=9) return -1;
  if (bcl_desktop_test_port_monitor_modes()!=7) return -1;
  int checks=0;
  for (unsigned pair=1;pair<=4;pair++) {
    for (unsigned side=0;side<2;side++) {
      if (dt_pair_index(pair,side)!=(int)((pair-1)*2+side)) return -1;
      checks++;
    }
  }
  if (dt_pair_index(0,0)!=-1 || dt_pair_index(5,0)!=-1 ||
      dt_pair_index(1,2)!=-1) return -1;
  return checks+3;
}
int bcl_desktop_test_metadata_guards(void) {
  int checks = 0;
  for (int mode = 0; mode < 6; mode++) {
    struct session s = {0}; s.desktop = desktop_new();
    if (!s.desktop) return -1;
    struct desktop_target *d = s.desktop;
    d->active = true; d->source.id = 7; d->expected_present = true;
    snprintf(d->expected_type, sizeof(d->expected_type), "Spa:Id");
    snprintf(d->expected, sizeof(d->expected), "123");
    if (mode == 0) dt_property(&s, 7, "target.object", "Spa:Id", "123");
    if (mode == 1) dt_property(&s, 7, "target.object", "Spa:Id", "124");
    if (mode == 2) dt_property(&s, 7, "target.object", NULL, NULL);
    if (mode == 3) dt_property(&s, 0, "default.audio.sink", "Spa:String:JSON", "{}");
    if (mode == 4) dt_property(&s, 7, "target.node", "Spa:Id", "9");
    if (mode == 5) dt_property(&s, 8, "target.object", "Spa:Id", "124");
    bool expected = mode > 0 && mode < 5;
    if (d->conflict != expected || (expected && bcl_desktop_stop(&s) != -1)) {
      free(d); return -1;
    }
    free(d); checks++;
  }
  return checks;
}
#endif
