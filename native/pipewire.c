/* Narrow synchronous FFI adapter. Registry events stay on one owned loop.
 * No registry.destroy(id): only retained, non-lingering creation resources
 * can be destroyed. Public PipeWire headers supply all ABI definitions. */
#include <inttypes.h>
#include <pipewire/pipewire.h>
#include <spa/utils/string.h>
#include <stdarg.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#if !PW_CHECK_VERSION(1, 0, 0)
#error PipeWire headers >= 1.0 are required
#endif

#define LIMIT 40960
#define OWNER "beacn.link.owner"
enum { NODE = 1, PORT, LINK, CLIENT };
struct session;
struct object {
  struct session *s;
  uint32_t id, kind;
  struct pw_properties *props;
  struct pw_proxy *proxy;
  struct spa_hook listener;
  bool present, info;
  int state, direction;
  uint32_t out_node, out_port, in_node, in_port;
};
struct created {
  struct session *s;
  struct pw_proxy *proxy;
  struct spa_hook listener, info_listener;
  uint32_t id, out_node, out_port, in_node, in_port;
  uint64_t serial;
  char owner[129];
  bool bound, removed, error, info;
  int state;
  struct created *next;
};
struct session {
  struct pw_main_loop *loop;
  struct pw_context *context;
  struct pw_core *core;
  struct pw_registry *registry;
  struct spa_hook core_listener, registry_listener;
  struct object *objects[LIMIT];
  size_t count;
  struct created *created, *pending;
  bool deleting, fatal, core_seen;
  uint32_t cookie;
  char name[129];
  int seq, done;
};
struct endpoint {
  uint32_t id;
  uint64_t serial;
  uint32_t node;
  uint64_t node_serial;
  int direction, channel;
};
struct receipt {
  uint32_t id;
  uint64_t serial;
};

static uint64_t num(const char *s) {
  if (!s || !*s)
    return 0;
  for (const char *p = s; *p; p++)
    if (*p < '0' || *p > '9')
      return 0;
  char *end;
  errno = 0;
  uint64_t n = strtoull(s, &end, 10);
  return errno || *end ? 0 : n;
}
static bool label(const char *s) {
  if (!s || !*s || strlen(s) > 128)
    return false;
  for (; *s; s++)
    if ((unsigned char)*s < 32 || (unsigned char)*s > 126)
      return false;
  return true;
}
static const char *prop(struct object *o, const char *key) {
  return pw_properties_get(o->props, key);
}
static uint64_t serial(struct object *o) {
  return num(prop(o, PW_KEY_OBJECT_SERIAL));
}
static struct object *find(struct session *s, uint32_t id, int kind) {
  for (size_t i = 0; i < s->count; i++)
    if (s->objects[i]->present && s->objects[i]->id == id &&
        (int)s->objects[i]->kind == kind)
      return s->objects[i];
  return NULL;
}
static bool audio(struct object *o) {
  const char *v = prop(o, PW_KEY_MEDIA_CLASS);
  return v &&
         (!strncmp(v, "Audio/", 6) || spa_streq(v, "Stream/Output/Audio") ||
          spa_streq(v, "Stream/Input/Audio"));
}
static void node_info(void *data, const struct pw_node_info *info) {
  struct object *o = data;
  if (info->props)
    pw_properties_update(o->props, info->props);
  o->state = info->state;
  o->info = true;
}
static void port_info(void *data, const struct pw_port_info *info) {
  struct object *o = data;
  if (info->props)
    pw_properties_update(o->props, info->props);
  o->direction = info->direction;
  o->info = true;
}
static void link_info(void *data, const struct pw_link_info *info) {
  struct object *o = data;
  if (info->props)
    pw_properties_update(o->props, info->props);
  o->state = info->state;
  o->out_node = info->output_node_id;
  o->out_port = info->output_port_id;
  o->in_node = info->input_node_id;
  o->in_port = info->input_port_id;
  o->info = true;
}
static void client_info(void *data, const struct pw_client_info *info) {
  struct object *o = data;
  if (info->props)
    pw_properties_update(o->props, info->props);
  o->info = true;
}
static const struct pw_node_events node_events = {
    .version = PW_VERSION_NODE_EVENTS, .info = node_info};
static const struct pw_port_events port_events = {
    .version = PW_VERSION_PORT_EVENTS, .info = port_info};
static const struct pw_link_events link_events = {
    .version = PW_VERSION_LINK_EVENTS, .info = link_info};
static const struct pw_client_events client_events = {
    .version = PW_VERSION_CLIENT_EVENTS, .info = client_info};
static void global(void *data, uint32_t id, uint32_t permissions,
                   const char *type, uint32_t version,
                   const struct spa_dict *props) {
  struct session *s = data;
  int kind;
  uint32_t v;
  const void *events;
  (void)permissions;
  if (spa_streq(type, PW_TYPE_INTERFACE_Device)) {
    s->fatal = true;
    return;
  }
  if (spa_streq(type, PW_TYPE_INTERFACE_Module)) {
    const char *name =
        props ? spa_dict_lookup(props, PW_KEY_MODULE_NAME) : NULL;
    const char *allowed[] = {"libpipewire-module-protocol-native",
                             "libpipewire-module-metadata",
                             "libpipewire-module-spa-node-factory",
                             "libpipewire-module-client-node",
                             "libpipewire-module-adapter",
                             "libpipewire-module-link-factory",
                             "libpipewire-module-access",
                             NULL};
    for (int i = 0; allowed[i]; i++)
      if (spa_streq(name, allowed[i]))
        return;
    s->fatal = true;
    return;
  }
  if (spa_streq(type, PW_TYPE_INTERFACE_Node)) {
    kind = NODE;
    v = PW_VERSION_NODE;
    events = &node_events;
  } else if (spa_streq(type, PW_TYPE_INTERFACE_Port)) {
    kind = PORT;
    v = PW_VERSION_PORT;
    events = &port_events;
  } else if (spa_streq(type, PW_TYPE_INTERFACE_Link)) {
    kind = LINK;
    v = PW_VERSION_LINK;
    events = &link_events;
  } else if (spa_streq(type, PW_TYPE_INTERFACE_Client)) {
    kind = CLIENT;
    v = PW_VERSION_CLIENT;
    events = &client_events;
  } else
    return;
  if (s->count == LIMIT || !props || find(s, id, kind)) {
    s->fatal = true;
    return;
  }
  struct object *o = calloc(1, sizeof(*o));
  if (!o) {
    s->fatal = true;
    return;
  }
  o->s = s;
  o->id = id;
  o->kind = kind;
  o->present = true;
  o->props = pw_properties_new_dict(props);
  if (!o->props) {
    free(o);
    s->fatal = true;
    return;
  }
  o->proxy = pw_registry_bind(s->registry, id, type, SPA_MIN(v, version), 0);
  if (!o->proxy) {
    pw_properties_free(o->props);
    free(o);
    s->fatal = true;
    return;
  }
  s->objects[s->count++] = o;
  pw_proxy_add_object_listener(o->proxy, &o->listener, events, o);
}
static void global_remove(void *data, uint32_t id) {
  struct session *s = data;
  for (size_t i = 0; i < s->count; i++)
    if (s->objects[i]->id == id && s->objects[i]->present)
      s->objects[i]->present = false;
}
static const struct pw_registry_events registry_events = {
    .version = PW_VERSION_REGISTRY_EVENTS,
    .global = global,
    .global_remove = global_remove};
static void core_info(void *data, const struct pw_core_info *info) {
  struct session *s = data;
  if (info->cookie != s->cookie || !spa_streq(info->name, s->name))
    s->fatal = true;
  s->core_seen = true;
}
static void done(void *data, uint32_t id, int seq) {
  struct session *s = data;
  if (id == PW_ID_CORE)
    s->done = seq;
}
static void core_error(void *data, uint32_t id, int seq, int res,
                       const char *message) {
  struct session *s = data;
  (void)seq;
  (void)res;
  (void)message;
  if (s->pending && s->pending->proxy &&
      id == pw_proxy_get_id(s->pending->proxy))
    s->pending->error = true;
  else {
    /* Registry enumeration can race a short-lived query client's departure.
     * These proxies only receive a bind request, never mutation requests.
     * An ENOENT for that exact local proxy means its observed global vanished.
     * Any surviving playback reference to it still makes snapshot fail. */
    for (size_t i = 0; i < s->count; i++) {
      struct object *o = s->objects[i];
      if (res == -ENOENT && o->proxy && id == pw_proxy_get_id(o->proxy)) {
        o->present = false;
        return;
      }
    }
    fprintf(stderr, "native core error: object=%u sequence=%d result=%d\n", id,
            seq, res);
    s->fatal = true;
  }
}
static const struct pw_core_events core_events = {.version =
                                                      PW_VERSION_CORE_EVENTS,
                                                  .info = core_info,
                                                  .done = done,
                                                  .error = core_error};
static int64_t millis(void) {
  struct timespec t;
  clock_gettime(CLOCK_MONOTONIC, &t);
  return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
static int barrier(struct session *s) {
  if (s->fatal)
    return -1;
  s->seq = pw_core_sync(s->core, PW_ID_CORE, s->seq);
  if (s->seq < 0) {
    s->fatal = true;
    return -1;
  }
  int64_t deadline = millis() + 5000;
  struct pw_loop *loop = pw_main_loop_get_loop(s->loop);
  while (s->done != s->seq && !s->fatal && millis() < deadline) {
    if (pw_loop_iterate(loop, 50) < 0) {
      s->fatal = true;
      return -1;
    }
  }
  return s->fatal || s->done != s->seq ? -1 : 0;
}
void bcl_close(struct session *s) {
  if (!s)
    return;
  /* Release every local proxy/listener before the core. Destroying a query
   * proxy only releases its binding; it never destroys a foreign global. */
  for (struct created *c = s->created; c; c = c->next) {
    if (c->proxy) {
      spa_hook_remove(&c->listener);
      spa_hook_remove(&c->info_listener);
      pw_proxy_destroy(c->proxy);
      c->proxy = NULL;
    }
  }
  for (size_t i = 0; i < s->count; i++) {
    struct object *o = s->objects[i];
    if (o->proxy) {
      spa_hook_remove(&o->listener);
      pw_proxy_destroy(o->proxy);
      o->proxy = NULL;
    }
  }
  if (s->registry) {
    spa_hook_remove(&s->registry_listener);
    pw_proxy_destroy((struct pw_proxy *)s->registry);
  }
  /* Disconnect releases only this client's non-lingering creation resources. */
  if (s->core) {
    spa_hook_remove(&s->core_listener);
    pw_core_disconnect(s->core);
  }
  if (s->context)
    pw_context_destroy(s->context);
  if (s->loop)
    pw_main_loop_destroy(s->loop);
  for (size_t i = 0; i < s->count; i++) {
    pw_properties_free(s->objects[i]->props);
    free(s->objects[i]);
  }
  struct created *c = s->created;
  while (c) {
    struct created *next = c->next;
    free(c);
    c = next;
  }
  free(s);
}
struct session *bcl_open(int fd, const char *config, uint32_t cookie,
                         const char *name, uint32_t server_pid) {
  struct ucred peer;
  socklen_t len = sizeof(peer);
  if (!label(name) ||
      getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &peer, &len) < 0 ||
      peer.uid != getuid() || peer.pid != (pid_t)server_pid) {
    close(fd);
    return NULL;
  }
  /* Context's explicit config must not be redirected by environment overrides.
   */
  const char *vars[] = {"PIPEWIRE_CONFIG_NAME",
                        "PIPEWIRE_CONFIG_DIR",
                        "PIPEWIRE_CONFIG_PREFIX",
                        "PIPEWIRE_CONFIG_OVERRIDE_NAME",
                        "PIPEWIRE_CONFIG_OVERRIDE_PREFIX",
                        NULL};
  for (int i = 0; vars[i]; i++)
    if (getenv(vars[i])) {
      close(fd);
      return NULL;
    }
  pw_init(NULL, NULL);
  struct session *s = calloc(1, sizeof(*s));
  if (!s) {
    close(fd);
    return NULL;
  }
  s->cookie = cookie;
  snprintf(s->name, sizeof(s->name), "%s", name);
  s->done = -1;
  s->loop = pw_main_loop_new(NULL);
  if (!s->loop) {
    close(fd);
    bcl_close(s);
    return NULL;
  }
  s->context =
      pw_context_new(pw_main_loop_get_loop(s->loop),
                     pw_properties_new(PW_KEY_CONFIG_NAME, config,
                                       "context.num-data-loops", "0", NULL),
                     0);
  if (!s->context) {
    close(fd);
    bcl_close(s);
    return NULL;
  }
  s->core = pw_context_connect_fd(
      s->context, fd,
      pw_properties_new(PW_KEY_APP_NAME, "beacn-private-executor", NULL), 0);
  if (!s->core) {
    bcl_close(s);
    return NULL;
  }
  pw_core_add_listener(s->core, &s->core_listener, &core_events, s);
  s->registry = pw_core_get_registry(s->core, PW_VERSION_REGISTRY, 0);
  if (!s->registry) {
    bcl_close(s);
    return NULL;
  }
  pw_registry_add_listener(s->registry, &s->registry_listener, &registry_events,
                           s);
  /* Second barrier includes info events resulting from registry binds. */
  if (barrier(s) < 0 || barrier(s) < 0 || !s->core_seen) {
    bcl_close(s);
    return NULL;
  }
  return s;
}
static int append(char **buf, size_t *left, const char *fmt, ...) {
  va_list ap;
  va_start(ap, fmt);
  int n = vsnprintf(*buf, *left, fmt, ap);
  va_end(ap);
  if (n < 0 || (size_t)n >= *left)
    return -1;
  *buf += n;
  *left -= n;
  return 0;
}
#define ROW(...)                                                               \
  do {                                                                         \
    if (append(&buf, &left, __VA_ARGS__) < 0)                                  \
      return -1;                                                               \
  } while (0)
int bcl_snapshot(struct session *s, char *buf, size_t left) {
  if (barrier(s) < 0 || barrier(s) < 0) {
    fprintf(stderr, "native snapshot: barrier failed\n");
    return -1;
  }
  ROW("core\tpw-core:%" PRIu32 "\n", s->cookie);
  for (size_t i = 0; i < s->count; i++) {
    struct object *o = s->objects[i];
    if (!o->present)
      continue;
    if (o->kind == NODE && audio(o)) {
      if (!o->info || !serial(o)) {
        fprintf(stderr, "native snapshot: node metadata incomplete\n");
        return -1;
      }
      ROW("node\t%" PRIu32 "\t%" PRIu64 "\n", o->id, serial(o));
      if (spa_streq(prop(o, PW_KEY_MEDIA_CLASS), "Stream/Output/Audio")) {
        uint64_t cid = num(prop(o, PW_KEY_CLIENT_ID));
        if (cid > UINT32_MAX)
          return -1;
        struct object *c = find(s, (uint32_t)cid, CLIENT);
        if (!c || !c->info || !serial(c))
          return -1;
        const char *pid = prop(o, PW_KEY_APP_PROCESS_ID);
        if (!pid)
          pid = prop(c, PW_KEY_APP_PROCESS_ID);
        uint64_t p = num(pid);
        if (!p || p > UINT32_MAX)
          return -1;
        const char *state = o->state == PW_NODE_STATE_RUNNING ||
                                    o->state == PW_NODE_STATE_IDLE ||
                                    o->state == PW_NODE_STATE_SUSPENDED
                                ? "live"
                                : "stale";
        ROW("playback\tpw-client:%" PRIu64 ":pid:%" PRIu64 "\tpw-node:%" PRIu64
            "\t%s\t%" PRIu32 "\t%" PRIu64 "\n",
            serial(c), p, serial(o), state, o->id, serial(o));
      }
    } else if (o->kind == PORT) {
      uint64_t nid = num(prop(o, PW_KEY_NODE_ID));
      if (nid > UINT32_MAX)
        return -1;
      struct object *node = find(s, (uint32_t)nid, NODE);
      if (!node || !audio(node))
        continue;
      const char *dir = prop(o, PW_KEY_PORT_DIRECTION),
                 *ch = prop(o, PW_KEY_AUDIO_CHANNEL),
                 *control = prop(o, PW_KEY_PORT_CONTROL);
      if (!o->info || !serial(o) || !serial(node) || !label(ch) ||
          (control && !spa_streq(control, "false")))
        return -1;
      if (!dir ||
          (spa_streq(dir, "out")
               ? o->direction != PW_DIRECTION_OUTPUT
               : !spa_streq(dir, "in") || o->direction != PW_DIRECTION_INPUT))
        return -1;
      ROW("port\t%" PRIu32 "\t%" PRIu64 "\t%" PRIu32 "\t%" PRIu64 "\t%s\t%s\n",
          o->id, serial(o), node->id, serial(node), dir,
          spa_streq(ch, "FL")   ? "FL"
          : spa_streq(ch, "FR") ? "FR"
                                : "other");
    } else if (o->kind == LINK) {
      if (!o->info) {
        fprintf(stderr, "native snapshot: link info incomplete\n");
        return -1;
      }
      struct object *on = find(s, o->out_node, NODE),
                    *in = find(s, o->in_node, NODE);
      if ((!on || !audio(on)) && (!in || !audio(in)))
        continue;
      struct object *op = find(s, o->out_port, PORT),
                    *ip = find(s, o->in_port, PORT);
      if (!on || !in || !op || !ip || !audio(on) || !audio(in) || !serial(o) ||
          !serial(op) || !serial(ip) ||
          num(prop(op, PW_KEY_NODE_ID)) != on->id ||
          num(prop(ip, PW_KEY_NODE_ID)) != in->id)
        return -1;
      const char *owner = prop(o, OWNER);
      if (owner && !label(owner))
        return -1;
      ROW("link\t%" PRIu32 "\t%" PRIu64 "\t%" PRIu32 "\t%" PRIu64 "\t%" PRIu32
          "\t%" PRIu64 "\t%s\n",
          o->id, serial(o), op->id, serial(op), ip->id, serial(ip),
          owner ? owner : "");
    }
  }
  return 0;
}
static bool endpoint(struct session *s, const struct endpoint *e,
                     const char *owner) {
  struct object *p = find(s, e->id, PORT), *n = find(s, e->node, NODE);
  const char *ch = p ? prop(p, PW_KEY_AUDIO_CHANNEL) : NULL;
  return p && n && p->info && n->info && serial(p) == e->serial &&
         serial(n) == e->node_serial &&
         num(prop(p, PW_KEY_NODE_ID)) == e->node &&
         p->direction == e->direction && ch &&
         spa_streq(ch, e->channel == 0 ? "FL" : "FR") &&
         spa_streq(prop(n, "beacn.test.token"), owner) &&
         !prop(n, PW_KEY_DEVICE_ID);
}
static void created_bound(void *data, uint32_t id) {
  struct created *c = data;
  c->id = id;
  c->bound = true;
}
static void created_removed(void *data) {
  struct created *c = data;
  c->removed = true;
}
static void created_error(void *data, int seq, int res, const char *message) {
  struct created *c = data;
  (void)seq;
  (void)res;
  (void)message;
  c->error = true;
}
static void created_info(void *data, const struct pw_link_info *info) {
  struct created *c = data;
  c->info = true;
  c->state = info->state;
  if (info->output_node_id != c->out_node ||
      info->output_port_id != c->out_port ||
      info->input_node_id != c->in_node || info->input_port_id != c->in_port)
    c->error = true;
}
static const struct pw_proxy_events created_events = {
    .version = PW_VERSION_PROXY_EVENTS,
    .bound = created_bound,
    .removed = created_removed,
    .error = created_error};
static const struct pw_link_events created_link_events = {
    .version = PW_VERSION_LINK_EVENTS, .info = created_info};
/* 0 acknowledged, 1 proven no effect, -2 unresolved. Pending survives failures.
 */
int bcl_reconcile(struct session *s, struct receipt *result) {
  struct created *c = s->pending;
  if (!c || barrier(s) < 0 || barrier(s) < 0)
    return -2;
  if (s->deleting) {
    struct object *o = find(s, c->id, LINK);
    if (c->removed && (!o || serial(o) != c->serial)) {
      result->id = c->id;
      result->serial = c->serial;
      s->pending = NULL;
      return 0;
    }
    return -2;
  }
  struct object *o = c->bound ? find(s, c->id, LINK) : NULL;
  if (o && o->info && c->info && !c->removed && !c->error &&
      c->state >= PW_LINK_STATE_PAUSED && o->state >= PW_LINK_STATE_PAUSED &&
      serial(o) && o->out_node == c->out_node && o->out_port == c->out_port &&
      o->in_node == c->in_node && o->in_port == c->in_port &&
      spa_streq(prop(o, OWNER), c->owner)) {
    c->serial = serial(o);
    result->id = c->id;
    result->serial = c->serial;
    s->pending = NULL;
    return 0;
  }
  if (c->removed && !o) {
    s->pending = NULL;
    return 1;
  }
  return -2;
}
/* Guards must have just been validated against bcl_snapshot. No loop iteration
 * occurs between these local serial checks and request submission. Server-side
 * compare-and-swap is unavailable: post-operation endpoint checks remain
 * required. */
int bcl_perform(struct session *s, int deleting, const struct endpoint *out,
                const struct endpoint *in, const char *owner,
                const struct receipt *old, struct receipt *result) {
  if (s->fatal || s->pending || !endpoint(s, out, owner) ||
      !endpoint(s, in, owner) || !label(owner))
    return 1;
  struct created *c = NULL;
  if (deleting) {
    for (c = s->created; c; c = c->next)
      if (c->id == old->id && c->serial == old->serial && !c->removed &&
          c->proxy)
        break;
    struct object *o = find(s, old->id, LINK);
    if (!c || !o || serial(o) != old->serial || o->out_port != out->id ||
        o->in_port != in->id || !spa_streq(prop(o, OWNER), owner) ||
        !spa_streq(c->owner, owner))
      return 1;
    s->pending = c;
    s->deleting = true;
    pw_core_destroy(s->core, c->proxy);
  } else {
    for (size_t i = 0; i < s->count; i++) {
      struct object *o = s->objects[i];
      if (o->present && o->kind == LINK && o->out_port == out->id)
        return 1;
    }
    c = calloc(1, sizeof(*c));
    if (!c)
      return 1;
    c->s = s;
    c->out_node = out->node;
    c->out_port = out->id;
    c->in_node = in->node;
    c->in_port = in->id;
    snprintf(c->owner, sizeof(c->owner), "%s", owner);
    struct pw_properties *props =
        pw_properties_new(PW_KEY_OBJECT_LINGER, "false", OWNER, owner, NULL);
    if (!props) {
      free(c);
      return 1;
    }
    pw_properties_setf(props, PW_KEY_LINK_OUTPUT_PORT, "%u", out->id);
    pw_properties_setf(props, PW_KEY_LINK_INPUT_PORT, "%u", in->id);
    c->proxy =
        pw_core_create_object(s->core, "link-factory", PW_TYPE_INTERFACE_Link,
                              PW_VERSION_LINK, &props->dict, 0);
    pw_properties_free(props);
    if (!c->proxy) {
      free(c);
      return 1;
    }
    c->next = s->created;
    s->created = c;
    s->pending = c;
    s->deleting = false;
    pw_proxy_add_listener(c->proxy, &c->listener, &created_events, c);
    pw_proxy_add_object_listener(c->proxy, &c->info_listener,
                                 &created_link_events, c);
  }
  int64_t deadline = millis() + 5000;
  do {
    int r = bcl_reconcile(s, result);
    if (r != -2)
      return r;
    if (s->fatal)
      return -2;
    if (pw_loop_iterate(pw_main_loop_get_loop(s->loop), 50) < 0)
      return -2;
  } while (millis() < deadline);
  return -2;
}
