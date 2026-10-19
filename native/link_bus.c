/* Persistent BEACN Link channel bus (send and return pairs).
 *
 * LinX mirrors BEACN Link on Windows, where the Studio driver exposes four
 * permanent "Link N Out" playback endpoints and applications are routed by
 * pointing them at one of those endpoints. Here LinX owns four persistent
 * stereo sinks ("LinX Link Out", "LinX Link 2..4 Out", the Windows driver's
 * names on the gaming PC; node names linx_link_in_N stay stable) for the
 * lifetime of a connection and
 * bridges each sink's monitor to its Studio AUX pair through one retained
 * 96-kHz adapter. Applications are routed only by writing their stream's
 * `target.object` in the session manager's default metadata, exactly like a
 * desktop volume control. Ordinary desktop events (other applications,
 * default-device changes, idle streams, capture taps) never stop the bus.
 *
 * Ownership rules:
 * - Only proxies created by this client are destroyed; every created object is
 *   non-lingering, so a crash or disconnect releases it server-side.
 * - Metadata writes target only Stream/Output/Audio nodes that are present in
 *   the current registry and are not marked node.dont-move.
 * - The loop is a pw_thread_loop: every public entry point locks it.
 * - Anything owned that disappears or fails is recreated with backoff.
 */
#include <errno.h>
#include <float.h>
#include <inttypes.h>
#include <math.h>
#include <pipewire/extensions/metadata.h>
#include <pipewire/filter.h>
#include <pipewire/pipewire.h>
#include <spa/param/audio/format-utils.h>
#include <spa/param/port-config.h>
#include <spa/param/profile.h>
#include <spa/param/props.h>
#include <spa/pod/parser.h>
#include <spa/utils/json.h>
#include <spa/utils/string.h>
#include <stdarg.h>
#include <stdatomic.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#if !PW_CHECK_VERSION(1, 0, 0)
#error PipeWire headers >= 1.0 are required
#endif

#define LB_CHANNELS 4
/* Metered stereo pairs: Link In 1..4, then Link Out 1..4. */
#define LB_METERS (LB_CHANNELS * 2)
#define LB_OWNER "linx.link-bus"
#define LB_BACKOFF_MIN_MS 1000
#define LB_BACKOFF_MAX_MS 15000

/* Shared with src/metering.rs (identical to native/meter.inc). */
struct meter_mailbox {
  _Atomic uint32_t valid, sequence;
  _Atomic uint32_t peak[2], power[2];
  _Atomic uint64_t frames;
};
_Static_assert(sizeof(struct meter_mailbox) == 32 &&
                   offsetof(struct meter_mailbox, frames) == 24,
               "Rust meter mailbox layout mismatch");

enum { LB_NODE = 1, LB_PORT, LB_LINK, LB_CLIENT, LB_DEVICE };

struct lb_object {
  uint32_t id, kind;
  uint64_t serial;
  struct pw_properties *props;
  struct pw_proxy *proxy; /* bound nodes/clients only (for info/state) */
  struct spa_hook listener;
  int state;
  bool info;
  /* ports */
  uint32_t node;
  int direction;
  /* links */
  uint32_t out_node, out_port, in_node, in_port;
  struct lb_object *next;
};

/* Current `target.object` metadata value of one stream subject. */
struct lb_target {
  uint32_t subject;
  char value[64];
  struct lb_target *next;
};

struct lb_owned {
  struct pw_proxy *proxy;
  struct spa_hook listener, object_listener;
  uint32_t id; /* global id once bound */
  bool bound, failed;
  int state;
  int64_t created_ms;
  /* Backoff across recreations; kept when the proxy is reset. */
  int64_t retry_ms;
  int failures;
  char error[128];
  /* Sink level as reported by PipeWire (owned sinks only). */
  bool level_known, mute;
  float volume;
};

struct lb_meter_side {
  float peak[2];
  double power[2];
};
struct lb_meter {
  struct pw_filter *filter;
  struct spa_hook listener;
  void *ports[LB_METERS * 2];
  struct meter_mailbox *mailbox[LB_METERS];
  uint64_t frames[LB_METERS];
  uint32_t window_frames;
  struct lb_meter_side window[LB_METERS];
};

struct link_bus {
  struct pw_thread_loop *loop;
  struct pw_context *context;
  struct pw_core *core;
  struct pw_registry *registry;
  struct spa_hook core_listener, registry_listener;
  struct pw_metadata *metadata;
  struct spa_hook metadata_listener;
  uint32_t metadata_id;
  struct lb_target *targets;
  /* The session manager's default output: the one in use and the one the
   * user chose (node.name; empty when unset). */
  char default_sink[256], configured_sink[256];
  struct lb_object *objects;
  bool broken;
  char error[256];
  int seq, done;
  char token[64];
  /* owned */
  struct lb_owned sinks[LB_CHANNELS];
  struct lb_owned adapter;
  struct lb_owned bridges[LB_CHANNELS * 2];
  struct lb_owned meter_links[LB_CHANNELS * 2];
  bool adapter_configured;
  int card;
  struct lb_meter meter;
  /* Link Out: the Studio's capture pairs as four LinX sources. */
  struct lb_owned sources[LB_CHANNELS];
  struct lb_owned capture;
  struct lb_owned out_links[LB_CHANNELS * 2];
  struct lb_owned out_meter_links[LB_CHANNELS * 2];
  bool capture_configured;
  /* The Studio card's session-manager profile. LinX switches it off (not
   * saved) while it owns the device and restores the previous one after. */
  struct pw_proxy *device;
  struct spa_hook device_listener;
  uint32_t device_id;
  int profile_index;     /* current, -1 unknown */
  char profile_name[64];
  int off_index;         /* the card's "off" profile, -1 unknown or none */
  int profile_restore;   /* profile to restore, -1 none */
  bool profile_asserted; /* switched off once for this card/device */
  int reasserts;
  int64_t device_bound_ms;
};

static int64_t lb_millis(void) {
  struct timespec t;
  clock_gettime(CLOCK_MONOTONIC, &t);
  return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
static uint64_t lb_num(const char *s) {
  if (!s || !*s) return 0;
  for (const char *p = s; *p; p++)
    if (*p < '0' || *p > '9') return 0;
  errno = 0;
  char *end;
  uint64_t n = strtoull(s, &end, 10);
  return errno || *end ? 0 : n;
}
static const char *lb_prop(struct lb_object *o, const char *key) {
  return o && o->props ? pw_properties_get(o->props, key) : NULL;
}
static void lb_fail(struct link_bus *b, const char *fmt, ...) {
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(b->error, sizeof(b->error), fmt, ap);
  va_end(ap);
}

/* ---------- metadata ---------- */
static void lb_targets_clear(struct link_bus *b, uint32_t subject, bool all) {
  struct lb_target **slot = &b->targets;
  while (*slot) {
    if (all || (*slot)->subject == subject) {
      struct lb_target *t = *slot;
      *slot = t->next;
      free(t);
    } else {
      slot = &(*slot)->next;
    }
  }
}
/* The "name" of a default-device value such as {"name":"alsa_output.x"}. */
static void lb_json_name(const char *value, char *out, size_t size) {
  out[0] = '\0';
  if (!value) return;
  struct spa_json it[2];
  char key[16];
  spa_json_init(&it[0], value, strlen(value));
  if (spa_json_enter_object(&it[0], &it[1]) <= 0) return;
  while (spa_json_get_string(&it[1], key, sizeof(key)) > 0) {
    if (spa_streq(key, "name")) {
      if (spa_json_get_string(&it[1], out, (int)size) <= 0) out[0] = '\0';
      return;
    }
    const char *skip;
    if (spa_json_next(&it[1], &skip) <= 0) return;
  }
}
static int lb_metadata_property(void *data, uint32_t subject, const char *key,
                                const char *type, const char *value) {
  struct link_bus *b = data;
  (void)type;
  if (subject == PW_ID_CORE) {
    if (!key || spa_streq(key, "default.audio.sink"))
      lb_json_name(key ? value : NULL, b->default_sink, sizeof(b->default_sink));
    if (!key || spa_streq(key, "default.configured.audio.sink"))
      lb_json_name(key ? value : NULL, b->configured_sink, sizeof(b->configured_sink));
    return 0;
  }
  if (!key) { /* subject cleared */
    lb_targets_clear(b, subject, false);
    return 0;
  }
  if (!spa_streq(key, "target.object")) return 0;
  lb_targets_clear(b, subject, false);
  if (value && strlen(value) < 64) {
    struct lb_target *t = calloc(1, sizeof(*t));
    if (t) {
      t->subject = subject;
      snprintf(t->value, sizeof(t->value), "%s", value);
      t->next = b->targets;
      b->targets = t;
    }
  }
  return 0;
}
static const struct pw_metadata_events lb_metadata_events = {
    .version = PW_VERSION_METADATA_EVENTS, .property = lb_metadata_property};

/* ---------- Studio card profile ---------- */
static void lb_device_param(void *data, int seq, uint32_t id, uint32_t index, uint32_t next,
                            const struct spa_pod *param) {
  struct link_bus *b = data;
  (void)seq; (void)index; (void)next;
  if ((id != SPA_PARAM_Profile && id != SPA_PARAM_EnumProfile) || !param) return;
  int32_t idx = -1;
  const char *name = NULL;
  if (spa_pod_parse_object(param, SPA_TYPE_OBJECT_ParamProfile, NULL,
                           SPA_PARAM_PROFILE_index, SPA_POD_Int(&idx),
                           SPA_PARAM_PROFILE_name, SPA_POD_OPT_String(&name)) < 0)
    return;
  if (id == SPA_PARAM_EnumProfile) {
    if (spa_streq(name, "off")) b->off_index = idx;
    return;
  }
  b->profile_index = idx;
  snprintf(b->profile_name, sizeof(b->profile_name), "%s", name ? name : "");
}
/* Re-read the profile (and the profile list) whenever the device reports
 * them changed, like wpctl/pw-cli: a subscription alone is not reliable for
 * devices owned by the session manager. */
static void lb_device_info(void *data, const struct pw_device_info *info) {
  struct link_bus *b = data;
  if (!b->device || !(info->change_mask & PW_DEVICE_CHANGE_MASK_PARAMS)) return;
  for (uint32_t i = 0; i < info->n_params; i++) {
    uint32_t id = info->params[i].id;
    if ((id != SPA_PARAM_Profile && id != SPA_PARAM_EnumProfile) ||
        !(info->params[i].flags & SPA_PARAM_INFO_READ))
      continue;
    if (info->params[i].user == 0 && b->profile_index >= 0 && b->off_index >= 0) continue;
    pw_device_enum_params((struct pw_device *)b->device, 0, id, 0, UINT32_MAX, NULL);
  }
}
static const struct pw_device_events lb_device_events = {
    .version = PW_VERSION_DEVICE_EVENTS, .info = lb_device_info, .param = lb_device_param};

/* Lock held. Ask the session manager for one profile without saving it, so
 * its stored choice stays the user's. */
static void lb_device_set_profile(struct link_bus *b, int index) {
  if (!b->device || index < 0) return;
  uint8_t buf[256];
  struct spa_pod_builder pb = SPA_POD_BUILDER_INIT(buf, sizeof(buf));
  struct spa_pod *param = spa_pod_builder_add_object(
      &pb, SPA_TYPE_OBJECT_ParamProfile, SPA_PARAM_Profile,
      SPA_PARAM_PROFILE_index, SPA_POD_Int(index),
      SPA_PARAM_PROFILE_save, SPA_POD_Bool(false));
  if (param) pw_device_set_param((struct pw_device *)b->device, SPA_PARAM_Profile, 0, param);
}
/* Lock held. Drop the bound Studio device; restore its profile when asked and
 * LinX changed it. */
static void lb_device_release(struct link_bus *b, bool restore) {
  if (b->device) {
    /* Only undo LinX's own change: a profile picked since is left alone. */
    if (restore && b->profile_restore >= 0 && b->off_index >= 0 &&
        b->profile_index == b->off_index)
      lb_device_set_profile(b, b->profile_restore);
    spa_hook_remove(&b->device_listener);
    pw_proxy_destroy(b->device);
  }
  b->device = NULL;
  b->device_id = 0;
  b->profile_index = -1;
  b->profile_name[0] = 0;
  b->off_index = -1;
  b->profile_restore = -1;
  b->profile_asserted = false;
  b->reasserts = 0;
}
/* The ALSA card's id ("Link" for the Studio), which session managers use in
 * the device's object.path ("alsa:acp:Link"). */
static bool lb_card_id(int card, char *out, size_t size) {
  char path[64];
  snprintf(path, sizeof(path), "/proc/asound/card%d/id", card);
  FILE *f = fopen(path, "re");
  if (!f) return false;
  bool ok = fgets(out, (int)size, f) != NULL;
  fclose(f);
  if (!ok) return false;
  out[strcspn(out, "\r\n")] = 0;
  return out[0] != 0;
}
static bool lb_is_card_device(struct lb_object *o, const char *id) {
  const char *path = lb_prop(o, PW_KEY_OBJECT_PATH);
  if (o->kind != LB_DEVICE || !spa_streq(lb_prop(o, "device.api"), "alsa") || !path ||
      strncmp(path, "alsa:", 5) != 0)
    return false;
  const char *last = strrchr(path, ':');
  return last && spa_streq(last + 1, id);
}
/* Lock held. Bind the session manager's device for the selected Studio card
 * and switch its profile off once, so its own Studio nodes release the
 * device to LinX. A profile the user picks afterwards is left alone. */
static void lb_device_own(struct link_bus *b) {
  if (b->card < 0) return;
  if (!b->device) {
    char id[64];
    if (!lb_card_id(b->card, id, sizeof(id))) return;
    for (struct lb_object *o = b->objects; o; o = o->next) {
      if (!lb_is_card_device(o, id)) continue;
      b->device = pw_registry_bind(b->registry, o->id, PW_TYPE_INTERFACE_Device,
                                   PW_VERSION_DEVICE, 0);
      if (!b->device) return;
      b->device_id = o->id;
      b->device_bound_ms = lb_millis();
      b->profile_index = b->off_index = -1;
      pw_proxy_add_object_listener(b->device, &b->device_listener, &lb_device_events, b);
      pw_device_enum_params((struct pw_device *)b->device, 0, SPA_PARAM_EnumProfile, 0,
                            UINT32_MAX, NULL);
      pw_device_enum_params((struct pw_device *)b->device, 0, SPA_PARAM_Profile, 0,
                            UINT32_MAX, NULL);
      return; /* the profiles arrive asynchronously */
    }
    return;
  }
  /* A card without an "off" profile is never switched. */
  if (b->profile_index < 0 || b->off_index < 0) return;
  bool off = b->profile_index == b->off_index;
  if (!b->profile_asserted) {
    b->profile_asserted = true;
    if (off) return; /* already off: nothing of ours to restore */
    b->profile_restore = b->profile_index;
    lb_device_set_profile(b, b->off_index);
    return;
  }
  /* The session manager may apply its saved profile just after LinX starts;
   * switch off again briefly, never later (then it is the user's choice). */
  if (!off && lb_millis() - b->device_bound_ms < 5000 && b->reasserts < 3) {
    b->reasserts++;
    b->profile_restore = b->profile_index;
    lb_device_set_profile(b, b->off_index);
  }
}
/* Lock held. While the Studio's session-manager nodes are being released,
 * hold back LinX's own transports instead of failing on a busy device. */
static bool lb_device_settling(struct link_bus *b) {
  if (!b->device) return false;
  if (b->off_index >= 0 && b->profile_index == b->off_index) return false;
  return lb_millis() - b->device_bound_ms < 3000;
}

/* ---------- registry cache ---------- */
static struct lb_object *lb_find(struct link_bus *b, uint32_t id) {
  for (struct lb_object *o = b->objects; o; o = o->next)
    if (o->id == id) return o;
  return NULL;
}
static void lb_object_free(struct lb_object *o) {
  if (o->proxy) {
    spa_hook_remove(&o->listener);
    pw_proxy_destroy(o->proxy);
  }
  pw_properties_free(o->props);
  free(o);
}
static void lb_node_info(void *data, const struct pw_node_info *info) {
  struct lb_object *o = data;
  if (info->props) pw_properties_update(o->props, info->props);
  o->state = info->state;
  o->info = true;
}
static const struct pw_node_events lb_node_events = {
    .version = PW_VERSION_NODE_EVENTS, .info = lb_node_info};
static void lb_client_info(void *data, const struct pw_client_info *info) {
  struct lb_object *o = data;
  if (info->props) pw_properties_update(o->props, info->props);
  o->info = true;
}
static const struct pw_client_events lb_client_events = {
    .version = PW_VERSION_CLIENT_EVENTS, .info = lb_client_info};

static void lb_global(void *data, uint32_t id, uint32_t permissions,
                      const char *type, uint32_t version,
                      const struct spa_dict *props) {
  struct link_bus *b = data;
  (void)permissions;
  if (spa_streq(type, PW_TYPE_INTERFACE_Metadata)) {
    if (!b->metadata && props &&
        spa_streq(spa_dict_lookup(props, PW_KEY_METADATA_NAME), "default")) {
      b->metadata = pw_registry_bind(b->registry, id, type,
                                     SPA_MIN(version, (uint32_t)PW_VERSION_METADATA), 0);
      b->metadata_id = id;
      if (b->metadata)
        pw_metadata_add_listener(b->metadata, &b->metadata_listener, &lb_metadata_events, b);
    }
    return;
  }
  uint32_t kind = spa_streq(type, PW_TYPE_INTERFACE_Node)     ? LB_NODE
                  : spa_streq(type, PW_TYPE_INTERFACE_Port)   ? LB_PORT
                  : spa_streq(type, PW_TYPE_INTERFACE_Link)   ? LB_LINK
                  : spa_streq(type, PW_TYPE_INTERFACE_Client) ? LB_CLIENT
                  : spa_streq(type, PW_TYPE_INTERFACE_Device) ? LB_DEVICE
                                                              : 0;
  if (!kind || !props || lb_find(b, id)) return;
  struct lb_object *o = calloc(1, sizeof(*o));
  if (!o) return;
  o->id = id;
  o->kind = kind;
  o->props = pw_properties_new_dict(props);
  if (!o->props) { free(o); return; }
  o->serial = lb_num(pw_properties_get(o->props, PW_KEY_OBJECT_SERIAL));
  if (kind == LB_PORT) {
    o->node = (uint32_t)lb_num(pw_properties_get(o->props, PW_KEY_NODE_ID));
    o->direction = spa_streq(pw_properties_get(o->props, PW_KEY_PORT_DIRECTION), "in")
                       ? PW_DIRECTION_INPUT : PW_DIRECTION_OUTPUT;
  } else if (kind == LB_LINK) {
    o->out_node = (uint32_t)lb_num(pw_properties_get(o->props, PW_KEY_LINK_OUTPUT_NODE));
    o->out_port = (uint32_t)lb_num(pw_properties_get(o->props, PW_KEY_LINK_OUTPUT_PORT));
    o->in_node = (uint32_t)lb_num(pw_properties_get(o->props, PW_KEY_LINK_INPUT_NODE));
    o->in_port = (uint32_t)lb_num(pw_properties_get(o->props, PW_KEY_LINK_INPUT_PORT));
  } else if (kind == LB_CLIENT) {
    /* Client info carries the process identity for streams that lack it. */
    o->proxy = pw_registry_bind(b->registry, id, type,
                                SPA_MIN(version, (uint32_t)PW_VERSION_CLIENT), 0);
    if (o->proxy)
      pw_proxy_add_object_listener(o->proxy, &o->listener, &lb_client_events, o);
  } else {
    const char *mc = pw_properties_get(o->props, PW_KEY_MEDIA_CLASS);
    /* Bind audio streams and sinks for application properties and state. */
    if (mc && (spa_streq(mc, "Stream/Output/Audio") || spa_streq(mc, "Audio/Sink"))) {
      o->proxy = pw_registry_bind(b->registry, id, type,
                                  SPA_MIN(version, (uint32_t)PW_VERSION_NODE), 0);
      if (o->proxy)
        pw_proxy_add_object_listener(o->proxy, &o->listener, &lb_node_events, o);
    }
  }
  o->next = b->objects;
  b->objects = o;
}

/* ---------- owned objects ---------- */
static void lb_owned_reset(struct lb_owned *x) {
  if (x->proxy) {
    spa_hook_remove(&x->object_listener);
    spa_hook_remove(&x->listener);
    pw_proxy_destroy(x->proxy);
  }
  x->proxy = NULL;
  x->id = 0;
  x->bound = x->failed = false;
  x->state = 0;
  x->level_known = false;
}
/* A failure (as opposed to an orderly removal) backs off exponentially. */
static void lb_owned_failed(struct lb_owned *x, const char *why) {
  if (why) snprintf(x->error, sizeof(x->error), "%s", why);
  x->failures++;
  int64_t delay = LB_BACKOFF_MIN_MS;
  for (int i = 1; i < x->failures && delay < LB_BACKOFF_MAX_MS; i++) delay *= 2;
  if (delay > LB_BACKOFF_MAX_MS) delay = LB_BACKOFF_MAX_MS;
  x->retry_ms = lb_millis() + delay;
  lb_owned_reset(x);
}
static void lb_owned_forget(struct lb_owned *x, uint32_t id) {
  /* The server removed an object we created: recreate it on the next pass. */
  if (x->bound && x->id == id) lb_owned_reset(x);
}
static void lb_global_remove(void *data, uint32_t id) {
  struct link_bus *b = data;
  struct lb_object **slot = &b->objects;
  while (*slot) {
    if ((*slot)->id == id) {
      struct lb_object *o = *slot;
      *slot = o->next;
      lb_object_free(o);
      break;
    }
    slot = &(*slot)->next;
  }
  lb_targets_clear(b, id, false);
  if (b->metadata && id == b->metadata_id) {
    spa_hook_remove(&b->metadata_listener);
    lb_targets_clear(b, 0, true);
    pw_proxy_destroy((struct pw_proxy *)b->metadata);
    b->metadata = NULL;
    b->metadata_id = 0;
    b->default_sink[0] = b->configured_sink[0] = '\0';
  }
  for (int i = 0; i < LB_CHANNELS; i++) {
    lb_owned_forget(&b->sinks[i], id);
    lb_owned_forget(&b->sources[i], id);
  }
  for (int i = 0; i < LB_CHANNELS * 2; i++) {
    lb_owned_forget(&b->bridges[i], id);
    lb_owned_forget(&b->meter_links[i], id);
    lb_owned_forget(&b->out_links[i], id);
    lb_owned_forget(&b->out_meter_links[i], id);
  }
  if (b->adapter.bound && b->adapter.id == id) {
    lb_owned_forget(&b->adapter, id);
    b->adapter_configured = false;
  }
  if (b->capture.bound && b->capture.id == id) {
    lb_owned_forget(&b->capture, id);
    b->capture_configured = false;
  }
  if (b->device && id == b->device_id) lb_device_release(b, false);
}
static const struct pw_registry_events lb_registry_events = {
    .version = PW_VERSION_REGISTRY_EVENTS,
    .global = lb_global,
    .global_remove = lb_global_remove};

/* ---------- core ---------- */
static void lb_owned_error(struct lb_owned *x, uint32_t id, int res, const char *message) {
  if (x->proxy && pw_proxy_get_id(x->proxy) == id) {
    x->failed = true;
    snprintf(x->error, sizeof(x->error), "%s (%d)", message ? message : "error", res);
  }
}
static void lb_core_done(void *data, uint32_t id, int seq) {
  struct link_bus *b = data;
  if (id == PW_ID_CORE) {
    b->done = seq;
    pw_thread_loop_signal(b->loop, false);
  }
}
static void lb_core_error(void *data, uint32_t id, int seq, int res,
                          const char *message) {
  struct link_bus *b = data;
  (void)seq;
  if (id == PW_ID_CORE) {
    if (res == -EPIPE) {
      b->broken = true;
      lb_fail(b, "PipeWire connection lost: %s", message ? message : "");
    }
    pw_thread_loop_signal(b->loop, false);
    return;
  }
  for (int i = 0; i < LB_CHANNELS; i++) {
    lb_owned_error(&b->sinks[i], id, res, message);
    lb_owned_error(&b->sources[i], id, res, message);
  }
  for (int i = 0; i < LB_CHANNELS * 2; i++) {
    lb_owned_error(&b->bridges[i], id, res, message);
    lb_owned_error(&b->meter_links[i], id, res, message);
    lb_owned_error(&b->out_links[i], id, res, message);
    lb_owned_error(&b->out_meter_links[i], id, res, message);
  }
  lb_owned_error(&b->adapter, id, res, message);
  lb_owned_error(&b->capture, id, res, message);
}
static const struct pw_core_events lb_core_events = {
    .version = PW_VERSION_CORE_EVENTS, .done = lb_core_done, .error = lb_core_error};

/* Lock held. 0 when the server processed everything sent so far, -1 when the
 * connection is broken, -2 on a transient timeout. */
static int lb_sync(struct link_bus *b) {
  if (b->broken) return -1;
  int seq = pw_core_sync(b->core, PW_ID_CORE, b->seq);
  if (seq < 0) return -2;
  b->seq = seq;
  int64_t deadline = lb_millis() + 3000;
  while (b->done != b->seq && !b->broken) {
    if (lb_millis() > deadline) return -2;
    struct timespec abstime;
    pw_thread_loop_get_time(b->loop, &abstime, 200 * SPA_NSEC_PER_MSEC);
    pw_thread_loop_timed_wait_full(b->loop, &abstime);
  }
  return b->broken ? -1 : 0;
}

static void lb_owned_bound(void *data, uint32_t global_id) {
  struct lb_owned *x = data;
  x->id = global_id;
  x->bound = true;
}
static void lb_owned_removed(void *data) {
  struct lb_owned *x = data;
  x->failed = true;
}
static const struct pw_proxy_events lb_owned_events = {
    .version = PW_VERSION_PROXY_EVENTS, .bound = lb_owned_bound, .removed = lb_owned_removed};
static void lb_owned_node_info(void *data, const struct pw_node_info *info) {
  struct lb_owned *x = data;
  x->state = info->state;
  if (info->state == PW_NODE_STATE_ERROR) {
    x->failed = true;
    snprintf(x->error, sizeof(x->error), "%s", info->error ? info->error : "node error");
  } else if (info->state == PW_NODE_STATE_RUNNING) {
    x->failures = 0;
    x->error[0] = 0;
  }
}
/* Props echo: the sink's own channel volumes and mute, whoever changed them
 * (LinX, a desktop volume control, or session-manager restore). */
static void lb_owned_node_param(void *data, int seq, uint32_t id, uint32_t index,
                                uint32_t next, const struct spa_pod *param) {
  struct lb_owned *x = data;
  (void)seq; (void)index; (void)next;
  if (id != SPA_PARAM_Props || !param) return;
  struct spa_pod *volumes = NULL;
  bool mute = x->mute;
  if (spa_pod_parse_object(param, SPA_TYPE_OBJECT_Props, NULL,
                           SPA_PROP_channelVolumes, SPA_POD_OPT_Pod(&volumes),
                           SPA_PROP_mute, SPA_POD_OPT_Bool(&mute)) < 0)
    return;
  float values[SPA_AUDIO_MAX_CHANNELS];
  uint32_t n = volumes ? spa_pod_copy_array(volumes, SPA_TYPE_Float, values,
                                            SPA_AUDIO_MAX_CHANNELS) : 0;
  if (!n) return; /* not the main Props object */
  float sum = 0.f;
  for (uint32_t i = 0; i < n; i++) sum += isfinite(values[i]) ? values[i] : 0.f;
  x->volume = sum / (float)n;
  x->mute = mute;
  x->level_known = true;
}
static const struct pw_node_events lb_owned_node_events = {
    .version = PW_VERSION_NODE_EVENTS, .info = lb_owned_node_info,
    .param = lb_owned_node_param};
static void lb_owned_link_info(void *data, const struct pw_link_info *info) {
  struct lb_owned *x = data;
  x->state = info->state;
  if (info->state == PW_LINK_STATE_ERROR) {
    x->failed = true;
    snprintf(x->error, sizeof(x->error), "%s", info->error ? info->error : "link error");
  } else if (info->state == PW_LINK_STATE_ACTIVE) {
    x->failures = 0;
  }
}
static const struct pw_link_events lb_owned_link_events = {
    .version = PW_VERSION_LINK_EVENTS, .info = lb_owned_link_info};

static bool lb_owned_create(struct link_bus *b, struct lb_owned *x, const char *factory,
                            const char *type, uint32_t version, struct pw_properties *p) {
  if (!p) return false;
  pw_properties_set(p, PW_KEY_OBJECT_LINGER, "false");
  pw_properties_set(p, LB_OWNER, b->token);
  x->proxy = pw_core_create_object(b->core, factory, type, version, &p->dict, 0);
  pw_properties_free(p);
  if (!x->proxy) {
    lb_owned_failed(x, "create request failed");
    return false;
  }
  x->created_ms = lb_millis();
  x->bound = x->failed = false;
  pw_proxy_add_listener(x->proxy, &x->listener, &lb_owned_events, x);
  bool link = spa_streq(type, PW_TYPE_INTERFACE_Link);
  pw_proxy_add_object_listener(x->proxy, &x->object_listener,
                               link ? (const void *)&lb_owned_link_events
                                    : (const void *)&lb_owned_node_events,
                               x);
  if (!link) {
    uint32_t ids[] = {SPA_PARAM_Props};
    pw_node_subscribe_params((struct pw_node *)x->proxy, ids, 1);
  }
  return true;
}
/* Pass housekeeping: drop failed or never-bound proxies; true when (re)creation
 * is allowed now. */
static bool lb_owned_due(struct lb_owned *x) {
  if (x->proxy && x->failed) lb_owned_failed(x, NULL);
  else if (x->proxy && !x->bound && lb_millis() - x->created_ms > 5000)
    lb_owned_failed(x, "not acknowledged");
  return !x->proxy && lb_millis() >= x->retry_ms;
}
static struct lb_object *lb_owned_node(struct link_bus *b, struct lb_owned *x) {
  if (!x->bound || x->failed) return NULL;
  struct lb_object *o = lb_find(b, x->id);
  return o && o->kind == LB_NODE ? o : NULL;
}

/* Ports of a node by channel name; monitor selects sink monitor outputs. */
static struct lb_object *lb_port(struct link_bus *b, uint32_t node, int direction,
                                 const char *channel, bool monitor) {
  for (struct lb_object *o = b->objects; o; o = o->next) {
    if (o->kind != LB_PORT || o->node != node || o->direction != direction) continue;
    if (spa_streq(lb_prop(o, PW_KEY_PORT_MONITOR), "true") != monitor) continue;
    if (spa_streq(lb_prop(o, PW_KEY_AUDIO_CHANNEL), channel)) return o;
  }
  return NULL;
}
static struct lb_object *lb_link_between(struct link_bus *b, uint32_t out_port,
                                         uint32_t in_port) {
  for (struct lb_object *o = b->objects; o; o = o->next)
    if (o->kind == LB_LINK && o->out_port == out_port && o->in_port == in_port) return o;
  return NULL;
}

static void lb_create_sink(struct link_bus *b, int index) {
  char name[64], description[64], channel[16];
  snprintf(name, sizeof(name), "linx_link_in_%d", index + 1);
  /* Pair names as BEACN's Windows driver gives them on the gaming PC. */
  if (index == 0)
    snprintf(description, sizeof(description), "LinX Link Out");
  else
    snprintf(description, sizeof(description), "LinX Link %d Out", index + 1);
  snprintf(channel, sizeof(channel), "%d", index + 1);
  struct pw_properties *p = pw_properties_new(
      "factory.name", "support.null-audio-sink",
      PW_KEY_NODE_NAME, name, PW_KEY_NODE_DESCRIPTION, description,
      PW_KEY_NODE_NICK, description, PW_KEY_MEDIA_CLASS, "Audio/Sink",
      /* Not node.virtual: KDE hides virtual outputs by default, and these
       * are the outputs users pick to send audio to the streaming PC.
       * priority.session 0 keeps them from becoming an automatic default. */
      PW_KEY_NODE_AUTOCONNECT, "false",
      "node.driver", "true", "priority.driver", "100", "priority.session", "0",
      "audio.channels", "2", "audio.position", "[ FL FR ]",
      /* The Studio bridge reads the monitor, so the sink's own volume and
       * mute (LinX faders, desktop sound settings) control what is sent. */
      "monitor.channel-volumes", "true",
      "device.icon-name", "audio-card",
      "adapter.auto-port-config", "{ mode = dsp monitor = true position = preserve }",
      "linx.link-in", channel, NULL);
  lb_owned_create(b, &b->sinks[index], "adapter", PW_TYPE_INTERFACE_Node,
                  PW_VERSION_NODE, p);
}
static void lb_create_adapter(struct link_bus *b) {
  char name[64], path[32];
  snprintf(name, sizeof(name), "linx_studio_link_%u", (unsigned)getpid());
  snprintf(path, sizeof(path), "hw:%d,0", b->card);
  struct pw_properties *p = pw_properties_new(
      "factory.name", "api.alsa.pcm.sink",
      PW_KEY_NODE_NAME, name, PW_KEY_NODE_DESCRIPTION, "LinX Studio Link transport",
      /* Internal, like WirePlumber's split parents: desktop volume applets
       * list plain Audio/Sink nodes even when hidden, and a user could then
       * pick this transport (whose name changes per process) as the default. */
      PW_KEY_MEDIA_CLASS, "Audio/Sink/Internal", PW_KEY_NODE_AUTOCONNECT, "false",
      "node.driver", "true", "priority.driver", "3000", "node.pause-on-idle", "false",
      "api.alsa.path", path, "api.alsa.use-chmap", "false",
      "audio.rate", "96000", "audio.allowed-rates", "[ 96000 ]",
      "audio.format", "F32LE", "audio.channels", "8",
      "audio.position", "[ AUX0 AUX1 AUX2 AUX3 AUX4 AUX5 AUX6 AUX7 ]",
      /* Session-manager policy must not reconfigure or suspend this transport. */
      "wireplumber.is-virtual", "true", "node.hidden", "true",
      "state.restore-props", "false", "state.restore-target", "false",
      "priority.session", "0", NULL);
  pw_properties_setf(p, PW_KEY_NODE_GROUP, "linx-studio-%u", (unsigned)getpid());
  b->adapter_configured = false;
  lb_owned_create(b, &b->adapter, "adapter", PW_TYPE_INTERFACE_Node, PW_VERSION_NODE, p);
}
static void lb_create_source(struct link_bus *b, int index) {
  char name[64], description[64], channel[16];
  snprintf(name, sizeof(name), "linx_link_out_%d", index + 1);
  /* The Studio's return pairs, named as on Windows ("Link In"). */
  if (index == 0)
    snprintf(description, sizeof(description), "LinX Link In");
  else
    snprintf(description, sizeof(description), "LinX Link %d In", index + 1);
  snprintf(channel, sizeof(channel), "%d", index + 1);
  struct pw_properties *p = pw_properties_new(
      "factory.name", "support.null-audio-sink",
      PW_KEY_NODE_NAME, name, PW_KEY_NODE_DESCRIPTION, description,
      PW_KEY_NODE_NICK, description, PW_KEY_MEDIA_CLASS, "Audio/Source/Virtual",
      PW_KEY_NODE_AUTOCONNECT, "false", "priority.session", "0",
      "audio.channels", "2", "audio.position", "[ FL FR ]",
      "device.icon-name", "audio-input-microphone",
      "linx.link-out", channel, NULL);
  lb_owned_create(b, &b->sources[index], "adapter", PW_TYPE_INTERFACE_Node,
                  PW_VERSION_NODE, p);
}
static void lb_create_capture(struct link_bus *b) {
  char name[64], path[32];
  snprintf(name, sizeof(name), "linx_studio_link_out_%u", (unsigned)getpid());
  snprintf(path, sizeof(path), "hw:%d,0", b->card);
  struct pw_properties *p = pw_properties_new(
      "factory.name", "api.alsa.pcm.source",
      PW_KEY_NODE_NAME, name, PW_KEY_NODE_DESCRIPTION, "LinX Studio return transport",
      PW_KEY_MEDIA_CLASS, "Audio/Source/Internal", PW_KEY_NODE_AUTOCONNECT, "false",
      "node.driver", "true", "priority.driver", "2999", "node.pause-on-idle", "false",
      "api.alsa.path", path, "api.alsa.use-chmap", "false",
      "audio.rate", "96000", "audio.allowed-rates", "[ 96000 ]",
      "audio.format", "F32LE", "audio.channels", "8",
      "audio.position", "[ AUX0 AUX1 AUX2 AUX3 AUX4 AUX5 AUX6 AUX7 ]",
      "wireplumber.is-virtual", "true", "node.hidden", "true",
      "state.restore-props", "false", "state.restore-target", "false",
      "priority.session", "0", NULL);
  pw_properties_setf(p, PW_KEY_NODE_GROUP, "linx-studio-%u", (unsigned)getpid());
  b->capture_configured = false;
  lb_owned_create(b, &b->capture, "adapter", PW_TYPE_INTERFACE_Node, PW_VERSION_NODE, p);
}
/* Lock held. Expose the adapter's AUX channels as DSP ports (out: capture). */
static bool lb_configure_ports(struct lb_owned *x, enum spa_direction direction) {
  uint8_t fbuf[1024], buf[2048];
  struct spa_pod_builder fb = SPA_POD_BUILDER_INIT(fbuf, sizeof(fbuf));
  struct spa_pod_builder pb = SPA_POD_BUILDER_INIT(buf, sizeof(buf));
  struct spa_audio_info_raw info = {.format = SPA_AUDIO_FORMAT_F32P, .rate = 96000,
                                    .channels = 8};
  for (int i = 0; i < 8; i++) info.position[i] = SPA_AUDIO_CHANNEL_AUX0 + i;
  struct spa_pod *format = spa_format_audio_raw_build(&fb, SPA_PARAM_Format, &info);
  struct spa_pod *config = spa_pod_builder_add_object(
      &pb, SPA_TYPE_OBJECT_ParamPortConfig, SPA_PARAM_PortConfig,
      SPA_PARAM_PORT_CONFIG_direction, SPA_POD_Id(direction),
      SPA_PARAM_PORT_CONFIG_mode, SPA_POD_Id(SPA_PARAM_PORT_CONFIG_MODE_dsp),
      SPA_PARAM_PORT_CONFIG_monitor, SPA_POD_Bool(false),
      SPA_PARAM_PORT_CONFIG_control, SPA_POD_Bool(false),
      SPA_PARAM_PORT_CONFIG_format, SPA_POD_Pod(format));
  return config && x->proxy &&
         pw_node_set_param((struct pw_node *)x->proxy, SPA_PARAM_PortConfig, 0, config) >= 0;
}
static void lb_configure_adapter(struct link_bus *b) {
  uint8_t fbuf[1024], buf[2048];
  struct spa_pod_builder fb = SPA_POD_BUILDER_INIT(fbuf, sizeof(fbuf));
  struct spa_pod_builder pb = SPA_POD_BUILDER_INIT(buf, sizeof(buf));
  struct spa_audio_info_raw info = {.format = SPA_AUDIO_FORMAT_F32P, .rate = 96000,
                                    .channels = 8};
  for (int i = 0; i < 8; i++) info.position[i] = SPA_AUDIO_CHANNEL_AUX0 + i;
  struct spa_pod *format = spa_format_audio_raw_build(&fb, SPA_PARAM_Format, &info);
  struct spa_pod *config = spa_pod_builder_add_object(
      &pb, SPA_TYPE_OBJECT_ParamPortConfig, SPA_PARAM_PortConfig,
      SPA_PARAM_PORT_CONFIG_direction, SPA_POD_Id(SPA_DIRECTION_INPUT),
      SPA_PARAM_PORT_CONFIG_mode, SPA_POD_Id(SPA_PARAM_PORT_CONFIG_MODE_dsp),
      SPA_PARAM_PORT_CONFIG_monitor, SPA_POD_Bool(false),
      SPA_PARAM_PORT_CONFIG_control, SPA_POD_Bool(false),
      SPA_PARAM_PORT_CONFIG_format, SPA_POD_Pod(format));
  if (config && b->adapter.proxy &&
      pw_node_set_param((struct pw_node *)b->adapter.proxy, SPA_PARAM_PortConfig, 0,
                        config) >= 0)
    b->adapter_configured = true;
}
static void lb_ensure_link(struct link_bus *b, struct lb_owned *x, struct lb_object *out,
                           struct lb_object *in, bool passive) {
  /* A link whose endpoints changed is stale: replace it. */
  if (x->bound && !x->failed) {
    struct lb_object *l = lb_find(b, x->id);
    if (l && out && in && l->out_port == out->id && l->in_port == in->id) return;
    if (l && (!out || !in || l->out_port != out->id || l->in_port != in->id))
      lb_owned_reset(x);
  }
  if (!lb_owned_due(x) || x->proxy) return;
  if (!out || !in) return;
  if (lb_link_between(b, out->id, in->id)) return; /* already linked */
  struct pw_properties *p = pw_properties_new(PW_KEY_LINK_PASSIVE, passive ? "true" : "false",
                                              NULL);
  if (!p) return;
  pw_properties_setf(p, PW_KEY_LINK_OUTPUT_PORT, "%u", out->id);
  pw_properties_setf(p, PW_KEY_LINK_INPUT_PORT, "%u", in->id);
  pw_properties_setf(p, PW_KEY_LINK_OUTPUT_NODE, "%u", out->node);
  pw_properties_setf(p, PW_KEY_LINK_INPUT_NODE, "%u", in->node);
  lb_owned_create(b, x, "link-factory", PW_TYPE_INTERFACE_Link, PW_VERSION_LINK, p);
}

/* ---------- meter ---------- */
static void lb_meter_process(void *data, struct spa_io_position *position) {
  struct lb_meter *m = data;
  if (!position || !position->clock.duration || position->clock.duration > 8192) return;
  uint32_t n = (uint32_t)position->clock.duration;
  uint32_t rate = position->clock.rate.num ? position->clock.rate.denom / position->clock.rate.num : 0;
  if (rate < 8000 || rate > 384000) return;
  for (int c = 0; c < LB_METERS; c++) {
    for (int side = 0; side < 2; side++) {
      const float *in = pw_filter_get_dsp_buffer(m->ports[c * 2 + side], n);
      float peak = 0.f;
      double sum = 0.;
      if (in)
        for (uint32_t i = 0; i < n; i++) {
          float v = in[i];
          if (!isfinite(v)) v = 0.f;
          float a = v < 0.f ? -v : v;
          if (a > peak) peak = a;
          sum += (double)v * v;
        }
      if (peak > m->window[c].peak[side]) m->window[c].peak[side] = peak;
      m->window[c].power[side] += sum;
    }
    m->frames[c] += n;
  }
  m->window_frames += n;
  if (m->window_frames < rate / 25) return;
  for (int c = 0; c < LB_METERS; c++) {
    struct meter_mailbox *box = m->mailbox[c];
    if (box) {
      atomic_fetch_add(&box->sequence, 1);
      for (int side = 0; side < 2; side++) {
        float peak = m->window[c].peak[side];
        float power = (float)(m->window[c].power[side] / m->window_frames);
        uint32_t p, q;
        memcpy(&p, &peak, 4);
        memcpy(&q, &power, 4);
        atomic_store(&box->peak[side], p);
        atomic_store(&box->power[side], q);
      }
      atomic_store(&box->frames, m->frames[c]);
      atomic_fetch_add(&box->sequence, 1);
      atomic_store(&box->valid, 1);
    }
    memset(&m->window[c], 0, sizeof(m->window[c]));
  }
  m->window_frames = 0;
}
static const struct pw_filter_events lb_meter_events = {
    .version = PW_VERSION_FILTER_EVENTS, .process = lb_meter_process};

/* Meter port i: link<N>_FL/FR for Link In pairs, out<N>_FL/FR for Link Out. */
static void lb_meter_port_name(int i, char *port, size_t size) {
  int pair = i / 2;
  snprintf(port, size, "%s%d_%s", pair < LB_CHANNELS ? "link" : "out",
           pair % LB_CHANNELS + 1, i % 2 ? "FR" : "FL");
}
static bool lb_meter_start(struct link_bus *b) {
  struct lb_meter *m = &b->meter;
  char name[80];
  snprintf(name, sizeof(name), "linx_link_meter_%u", (unsigned)getpid());
  m->filter = pw_filter_new(b->core, "LinX Link meters",
      pw_properties_new(PW_KEY_NODE_NAME, name, PW_KEY_NODE_DESCRIPTION, "LinX Link levels",
                        PW_KEY_MEDIA_TYPE, "Audio", PW_KEY_MEDIA_CATEGORY, "Monitor",
                        PW_KEY_MEDIA_ROLE, "DSP", PW_KEY_NODE_AUTOCONNECT, "false",
                        PW_KEY_NODE_PASSIVE, "true", PW_KEY_NODE_VIRTUAL, "true",
                        "node.hidden", "true", "state.restore-props", "false",
                        "state.restore-target", "false", LB_OWNER, b->token, NULL));
  if (!m->filter) return false;
  pw_filter_add_listener(m->filter, &m->listener, &lb_meter_events, m);
  for (int i = 0; i < LB_METERS * 2; i++) {
    char port[32];
    lb_meter_port_name(i, port, sizeof(port));
    m->ports[i] = pw_filter_add_port(m->filter, PW_DIRECTION_INPUT,
        PW_FILTER_PORT_FLAG_MAP_BUFFERS, 0,
        pw_properties_new(PW_KEY_FORMAT_DSP, "32 bit float mono audio",
                          PW_KEY_PORT_NAME, port, PW_KEY_AUDIO_CHANNEL,
                          i % 2 ? "FR" : "FL", NULL),
        NULL, 0);
    if (!m->ports[i]) return false;
  }
  return pw_filter_connect(m->filter, PW_FILTER_FLAG_RT_PROCESS, NULL, 0) >= 0;
}
static void lb_meter_stop(struct link_bus *b) {
  struct lb_meter *m = &b->meter;
  /* pw_filter_destroy disconnects from the data loop (synchronized with the
   * realtime callback) before releasing its hooks; never edit them first. */
  if (m->filter) pw_filter_destroy(m->filter);
  m->filter = NULL;
  for (int c = 0; c < LB_METERS; c++)
    if (m->mailbox[c]) atomic_store(&m->mailbox[c]->valid, 0);
}
static struct lb_object *lb_meter_port(struct link_bus *b, int index) {
  uint32_t node = b->meter.filter ? pw_filter_get_node_id(b->meter.filter) : SPA_ID_INVALID;
  if (node == SPA_ID_INVALID) return NULL;
  char port[32];
  lb_meter_port_name(index, port, sizeof(port));
  for (struct lb_object *o = b->objects; o; o = o->next)
    if (o->kind == LB_PORT && o->node == node && o->direction == PW_DIRECTION_INPUT &&
        spa_streq(lb_prop(o, PW_KEY_PORT_NAME), port))
      return o;
  return NULL;
}

/* ---------- reconcile ---------- */
static void lb_reconcile(struct link_bus *b) {
  for (int i = 0; i < LB_CHANNELS; i++) {
    if (lb_owned_due(&b->sinks[i])) lb_create_sink(b, i);
    if (lb_owned_due(&b->sources[i])) lb_create_source(b, i);
  }
  lb_device_own(b);
  bool settling = lb_device_settling(b);
  if (b->card >= 0 && !settling) {
    if (lb_owned_due(&b->capture)) lb_create_capture(b);
    if (lb_owned_node(b, &b->capture) && !b->capture_configured)
      b->capture_configured = lb_configure_ports(&b->capture, SPA_DIRECTION_OUTPUT);
  } else if (b->card < 0 && b->capture.proxy) {
    lb_owned_reset(&b->capture);
    b->capture_configured = false;
  }
  struct lb_object *capture = lb_owned_node(b, &b->capture);
  for (int i = 0; i < LB_CHANNELS; i++) {
    struct lb_object *source = lb_owned_node(b, &b->sources[i]);
    for (int side = 0; side < 2; side++) {
      struct lb_object *out = NULL;
      if (capture && b->capture_configured) {
        char aux[8];
        snprintf(aux, sizeof(aux), "AUX%d", i * 2 + side);
        out = lb_port(b, capture->id, PW_DIRECTION_OUTPUT, aux, false);
      }
      struct lb_object *in =
          source ? lb_port(b, source->id, PW_DIRECTION_INPUT, side ? "FR" : "FL", false) : NULL;
      lb_ensure_link(b, &b->out_links[i * 2 + side], out, in, false);
      lb_ensure_link(b, &b->out_meter_links[i * 2 + side], out,
                     lb_meter_port(b, (LB_CHANNELS + i) * 2 + side), true);
    }
  }
  /* Studio transport: optional; recreated with backoff when busy, absent or
   * in error (unplug, xrun recovery failure). */
  if (b->card >= 0 && !settling) {
    if (lb_owned_due(&b->adapter)) lb_create_adapter(b);
    struct lb_object *adapter = lb_owned_node(b, &b->adapter);
    if (adapter && !b->adapter_configured) lb_configure_adapter(b);
  } else if (b->card < 0 && b->adapter.proxy) {
    lb_owned_reset(&b->adapter);
    b->adapter_configured = false;
  }
  struct lb_object *adapter = lb_owned_node(b, &b->adapter);
  for (int i = 0; i < LB_CHANNELS; i++) {
    struct lb_object *sink = lb_owned_node(b, &b->sinks[i]);
    for (int side = 0; side < 2; side++) {
      const char *ch = side ? "FR" : "FL";
      struct lb_object *out = sink ? lb_port(b, sink->id, PW_DIRECTION_OUTPUT, ch, true) : NULL;
      struct lb_object *in = NULL;
      if (adapter && b->adapter_configured) {
        char aux[8];
        snprintf(aux, sizeof(aux), "AUX%d", i * 2 + side);
        in = lb_port(b, adapter->id, PW_DIRECTION_INPUT, aux, false);
      }
      /* Non-passive: the Studio transport runs continuously while connected,
       * like the Windows endpoints, so pauses never re-prepare the device. */
      lb_ensure_link(b, &b->bridges[i * 2 + side], out, in, false);
      lb_ensure_link(b, &b->meter_links[i * 2 + side], out, lb_meter_port(b, i * 2 + side),
                     true);
    }
  }
}

/* ---------- public API ---------- */
/* mailboxes: LB_METERS entries, Link In 1..4 then Link Out 1..4 (or NULL). */
struct link_bus *lb_open(struct meter_mailbox **mailboxes) {
  pw_init(NULL, NULL);
  struct link_bus *b = calloc(1, sizeof(*b));
  if (!b) return NULL;
  b->card = -1;
  b->done = -1;
  b->profile_index = -1;
  b->off_index = -1;
  b->profile_restore = -1;
  snprintf(b->token, sizeof(b->token), "linx-%u", (unsigned)getpid());
  for (int c = 0; c < LB_METERS; c++) b->meter.mailbox[c] = mailboxes ? mailboxes[c] : NULL;
  b->loop = pw_thread_loop_new("linx-link-bus", NULL);
  if (!b->loop) { free(b); return NULL; }
  b->context = pw_context_new(pw_thread_loop_get_loop(b->loop),
                              pw_properties_new(PW_KEY_CONFIG_NAME, "client.conf", NULL), 0);
  if (!b->context) { pw_thread_loop_destroy(b->loop); free(b); return NULL; }
  if (pw_thread_loop_start(b->loop) < 0) {
    pw_context_destroy(b->context);
    pw_thread_loop_destroy(b->loop);
    free(b);
    return NULL;
  }
  pw_thread_loop_lock(b->loop);
  b->core = pw_context_connect(b->context,
                               pw_properties_new(PW_KEY_APP_NAME, "LinX",
                                                 PW_KEY_APP_ID, "io.github.linx", NULL), 0);
  if (!b->core) {
    lb_fail(b, "Cannot connect to PipeWire.");
    b->broken = true;
    pw_thread_loop_unlock(b->loop);
    return b;
  }
  pw_core_add_listener(b->core, &b->core_listener, &lb_core_events, b);
  b->registry = pw_core_get_registry(b->core, PW_VERSION_REGISTRY, 0);
  pw_registry_add_listener(b->registry, &b->registry_listener, &lb_registry_events, b);
  if (lb_sync(b) < 0 || lb_sync(b) < 0) {
    lb_fail(b, "PipeWire did not answer.");
    b->broken = true;
  }
  if (!b->broken && !lb_meter_start(b)) lb_fail(b, "Level meter unavailable.");
  if (!b->broken) lb_reconcile(b);
  pw_thread_loop_unlock(b->loop);
  return b;
}

void lb_close(struct link_bus *b) {
  if (!b) return;
  pw_thread_loop_lock(b->loop);
  lb_meter_stop(b);
  for (int i = 0; i < LB_CHANNELS * 2; i++) {
    lb_owned_reset(&b->meter_links[i]);
    lb_owned_reset(&b->bridges[i]);
    lb_owned_reset(&b->out_links[i]);
    lb_owned_reset(&b->out_meter_links[i]);
  }
  lb_owned_reset(&b->adapter);
  lb_owned_reset(&b->capture);
  for (int i = 0; i < LB_CHANNELS; i++) {
    lb_owned_reset(&b->sinks[i]);
    lb_owned_reset(&b->sources[i]);
  }
  /* Release the device first, then hand the profile back. */
  if (b->core && !b->broken) lb_sync(b);
  lb_device_release(b, !b->broken);
  if (b->core && !b->broken) lb_sync(b);
  if (b->metadata) {
    spa_hook_remove(&b->metadata_listener);
    pw_proxy_destroy((struct pw_proxy *)b->metadata);
    b->metadata = NULL;
  }
  lb_targets_clear(b, 0, true);
  while (b->objects) {
    struct lb_object *o = b->objects;
    b->objects = o->next;
    lb_object_free(o);
  }
  if (b->registry) {
    spa_hook_remove(&b->registry_listener);
    pw_proxy_destroy((struct pw_proxy *)b->registry);
  }
  if (b->core) {
    spa_hook_remove(&b->core_listener);
    pw_core_disconnect(b->core);
  }
  pw_thread_loop_unlock(b->loop);
  pw_thread_loop_stop(b->loop);
  pw_context_destroy(b->context);
  pw_thread_loop_destroy(b->loop);
  free(b);
}

/* Select the Studio ALSA card (-1 detaches the transport). */
void lb_set_card(struct link_bus *b, int card) {
  if (!b) return;
  pw_thread_loop_lock(b->loop);
  if (card != b->card) {
    lb_owned_reset(&b->adapter);
    b->adapter.failures = 0;
    b->adapter.retry_ms = 0;
    b->adapter.error[0] = 0;
    b->adapter_configured = false;
    lb_owned_reset(&b->capture);
    b->capture.failures = 0;
    b->capture.retry_ms = 0;
    b->capture.error[0] = 0;
    b->capture_configured = false;
    if (b->core && !b->broken) lb_sync(b);
    lb_device_release(b, !b->broken);
    b->card = card;
  }
  pw_thread_loop_unlock(b->loop);
}

/* Periodic self-healing pass: 0 ok, -1 connection lost, -2 transient stall. */
int lb_maintain(struct link_bus *b) {
  if (!b) return -1;
  pw_thread_loop_lock(b->loop);
  int result = lb_sync(b);
  if (result == 0) lb_reconcile(b);
  pw_thread_loop_unlock(b->loop);
  return result;
}

static bool lb_is_link_sink(struct link_bus *b, uint32_t node, int *channel) {
  for (int i = 0; i < LB_CHANNELS; i++)
    if (b->sinks[i].bound && !b->sinks[i].failed && b->sinks[i].id == node) {
      *channel = i + 1;
      return true;
    }
  return false;
}
/* Channel a stream plays to: only links into audio sinks count, so capture
 * taps (OBS application audio, volume meters) never hide the route. Returns
 * 1..4 when every sink link goes to one LinX channel, else 0. */
static int lb_stream_channel(struct link_bus *b, struct lb_object *stream) {
  int channel = 0;
  for (struct lb_object *l = b->objects; l; l = l->next) {
    if (l->kind != LB_LINK || l->out_node != stream->id) continue;
    int found = 0;
    if (lb_is_link_sink(b, l->in_node, &found)) {
      if (channel && channel != found) return 0;
      channel = found;
      continue;
    }
    struct lb_object *in = lb_find(b, l->in_node);
    if (in && spa_streq(lb_prop(in, PW_KEY_MEDIA_CLASS), "Audio/Sink")) return 0;
  }
  return channel;
}
/* Whether LinX's metadata currently points this stream at a LinX channel. */
static int lb_stream_target_channel(struct link_bus *b, struct lb_object *stream) {
  for (struct lb_target *t = b->targets; t; t = t->next) {
    if (t->subject != stream->id) continue;
    uint64_t serial = lb_num(t->value);
    for (int i = 0; i < LB_CHANNELS; i++) {
      struct lb_object *sink = lb_owned_node(b, &b->sinks[i]);
      if (sink && serial && sink->serial == serial) return i + 1;
    }
    return 0;
  }
  return 0;
}
static void lb_hex(char **buf, size_t *left, const char *s) {
  if (!s) s = "";
  for (; *s && *left > 3; s++) {
    int n = snprintf(*buf, *left, "%02x", (unsigned char)*s);
    *buf += n;
    *left -= (size_t)n;
  }
}
static void lb_put(char **buf, size_t *left, const char *fmt, ...) {
  if (*left < 2) return;
  va_list ap;
  va_start(ap, fmt);
  int n = vsnprintf(*buf, *left, fmt, ap);
  va_end(ap);
  if (n < 0) return;
  if ((size_t)n >= *left) n = (int)*left - 1;
  *buf += n;
  *left -= (size_t)n;
}
/* Property from the stream node, falling back to its client. */
static const char *lb_identity(struct link_bus *b, struct lb_object *node, const char *key) {
  const char *v = lb_prop(node, key);
  if (v && *v) return v;
  struct lb_object *client = lb_find(b, (uint32_t)lb_num(lb_prop(node, PW_KEY_CLIENT_ID)));
  return client && client->kind == LB_CLIENT ? lb_prop(client, key) : NULL;
}

/* Text snapshot (NUL-terminated), one record per line, tab separated:
 *   bus <healthy> <metadata> <sinks_ready> <card> <transport> <links>
 *       <generation> <hex error>
 *     transport: 0 none, 1 starting, 2 ready (idle), 3 running, 4 error
 *   out <sources_ready> <transport> <links> <profile_owned> <hex profile>
 *       <hex error>   (Link Out: capture transport coded like the bus's)
 *   stream <node> <serial> <client> <pid> <state> <channel> <dont_move>
 *          <target_channel> <info> <hex app name> <hex binary> <hex media name>
 *          <hex node name>
 *   default <hex default output> <hex configured default output>  (node.name)
 *   output <node> <serial> <hex node name> <hex description>  (Audio/Sink)
 * Returns 0, -1 on failure, -2 when the buffer is too small. */
int lb_snapshot(struct link_bus *b, char *out, size_t size) {
  if (!b || !out || size < 64) return -1;
  pw_thread_loop_lock(b->loop);
  char *buf = out;
  size_t left = size;
  int ready = 0, links = 0;
  for (int i = 0; i < LB_CHANNELS; i++)
    if (lb_owned_node(b, &b->sinks[i])) ready++;
  for (int i = 0; i < LB_CHANNELS * 2; i++)
    if (b->bridges[i].bound && !b->bridges[i].failed) links++;
  struct lb_object *adapter = lb_owned_node(b, &b->adapter);
  int transport = 0;
  if (b->card >= 0) {
    if (b->adapter.error[0] && !adapter) transport = 4;
    else if (!adapter || !b->adapter_configured) transport = 1;
    else transport = b->adapter.state == PW_NODE_STATE_RUNNING ? 3 : 2;
  }
  /* Routing generation: changes when metadata or any sink is recreated, so
   * the engine re-applies remembered channels. */
  uint64_t generation = b->metadata_id;
  for (int i = 0; i < LB_CHANNELS; i++) {
    struct lb_object *sink = lb_owned_node(b, &b->sinks[i]);
    generation = generation * 1000003u + (sink ? sink->serial : 0);
  }
  lb_put(&buf, &left, "bus\t%d\t%d\t%d\t%d\t%d\t%d\t%" PRIu64 "\t", !b->broken,
         b->metadata != NULL, ready, b->card, transport, links, generation);
  const char *error = b->broken ? b->error
                      : b->adapter.error[0] ? b->adapter.error
                      : b->error;
  lb_hex(&buf, &left, error);
  lb_put(&buf, &left, "\n");
  {
    int sources = 0, out_links = 0;
    for (int i = 0; i < LB_CHANNELS; i++)
      if (lb_owned_node(b, &b->sources[i])) sources++;
    for (int i = 0; i < LB_CHANNELS * 2; i++)
      if (b->out_links[i].bound && !b->out_links[i].failed) out_links++;
    struct lb_object *capture = lb_owned_node(b, &b->capture);
    int out_transport = 0;
    if (b->card >= 0) {
      if (b->capture.error[0] && !capture) out_transport = 4;
      else if (!capture || !b->capture_configured) out_transport = 1;
      else out_transport = b->capture.state == PW_NODE_STATE_RUNNING ? 3 : 2;
    }
    lb_put(&buf, &left, "out\t%d\t%d\t%d\t%d\t", sources, out_transport, out_links,
           b->profile_restore >= 0 && b->off_index >= 0 &&
               b->profile_index == b->off_index);
    lb_hex(&buf, &left, b->profile_name);
    lb_put(&buf, &left, "\t");
    lb_hex(&buf, &left, b->capture.error);
    lb_put(&buf, &left, "\n");
  }
  for (int i = 0; i < LB_CHANNELS; i++) {
    struct lb_owned *x = &b->sinks[i];
    bool known = x->level_known && lb_owned_node(b, x);
    lb_put(&buf, &left, "level\t%d\t%d\t%.6f\t%d\n", i + 1, known,
           known ? (double)x->volume : 0.0, known && x->mute);
  }
  for (struct lb_object *o = b->objects; o; o = o->next) {
    if (o->kind != LB_NODE || !spa_streq(lb_prop(o, PW_KEY_MEDIA_CLASS), "Stream/Output/Audio"))
      continue;
    if (lb_prop(o, LB_OWNER)) continue;
    lb_put(&buf, &left, "stream\t%u\t%" PRIu64 "\t%" PRIu64 "\t%" PRIu64 "\t%d\t%d\t%d\t%d\t%d\t",
           o->id, o->serial, lb_num(lb_prop(o, PW_KEY_CLIENT_ID)),
           lb_num(lb_identity(b, o, PW_KEY_APP_PROCESS_ID)), o->state, lb_stream_channel(b, o),
           spa_streq(lb_prop(o, "node.dont-move"), "true"), lb_stream_target_channel(b, o),
           o->info);
    lb_hex(&buf, &left, lb_identity(b, o, PW_KEY_APP_NAME));
    lb_put(&buf, &left, "\t");
    lb_hex(&buf, &left, lb_identity(b, o, PW_KEY_APP_PROCESS_BINARY));
    lb_put(&buf, &left, "\t");
    lb_hex(&buf, &left, lb_prop(o, PW_KEY_MEDIA_NAME));
    lb_put(&buf, &left, "\t");
    lb_hex(&buf, &left, lb_prop(o, PW_KEY_NODE_NAME));
    lb_put(&buf, &left, "\n");
  }
  lb_put(&buf, &left, "default\t");
  lb_hex(&buf, &left, b->default_sink);
  lb_put(&buf, &left, "\t");
  lb_hex(&buf, &left, b->configured_sink);
  lb_put(&buf, &left, "\n");
  for (struct lb_object *o = b->objects; o; o = o->next) {
    if (o->kind != LB_NODE || !spa_streq(lb_prop(o, PW_KEY_MEDIA_CLASS), "Audio/Sink")) continue;
    const char *name = lb_prop(o, PW_KEY_NODE_NAME);
    if (!name || !*name) continue;
    const char *description = lb_prop(o, PW_KEY_NODE_DESCRIPTION);
    if (!description || !*description) description = lb_prop(o, PW_KEY_NODE_NICK);
    lb_put(&buf, &left, "output\t%u\t%" PRIu64 "\t", o->id, o->serial);
    lb_hex(&buf, &left, name);
    lb_put(&buf, &left, "\t");
    lb_hex(&buf, &left, description && *description ? description : name);
    lb_put(&buf, &left, "\n");
  }
  int result = left < 64 ? -2 : 0;
  pw_thread_loop_unlock(b->loop);
  return result;
}

/* Lock held. Validate a stream subject for a metadata write. */
static struct lb_object *lb_movable(struct link_bus *b, uint32_t node, uint64_t serial) {
  struct lb_object *o = lb_find(b, node);
  if (b->broken || !b->metadata || !o || o->kind != LB_NODE || o->serial != serial ||
      !spa_streq(lb_prop(o, PW_KEY_MEDIA_CLASS), "Stream/Output/Audio") ||
      spa_streq(lb_prop(o, "node.dont-move"), "true"))
    return NULL;
  return o;
}

/* Point one output stream at Link In 1..4, or 0 to clear LinX's target so it
 * follows the desktop default. The session manager performs the move. */
int lb_route(struct link_bus *b, uint32_t node, uint64_t serial, int channel) {
  if (!b || channel < 0 || channel > LB_CHANNELS) return -1;
  pw_thread_loop_lock(b->loop);
  int result = -1;
  if (lb_movable(b, node, serial)) {
    if (channel == 0) {
      result = pw_metadata_set_property(b->metadata, node, "target.object", NULL, NULL);
    } else {
      struct lb_object *sink = lb_owned_node(b, &b->sinks[channel - 1]);
      if (sink && sink->serial) {
        char value[32];
        snprintf(value, sizeof(value), "%" PRIu64, sink->serial);
        result = pw_metadata_set_property(b->metadata, node, "target.object", "Spa:Id", value);
      }
    }
    if (result >= 0) result = lb_sync(b);
  }
  pw_thread_loop_unlock(b->loop);
  return result < 0 ? -1 : 0;
}

static struct lb_object *lb_node_named(struct link_bus *b, const char *name) {
  for (struct lb_object *o = b->objects; o; o = o->next)
    if (o->kind == LB_NODE && spa_streq(lb_prop(o, PW_KEY_NODE_NAME), name)) return o;
  return NULL;
}

/* node.name of the output a stream is explicitly targeted at (its current
 * `target.object`), for restoring it later. Returns 0 and writes the name, or
 * -1 when the stream follows the default output. LinX's own sinks are never
 * reported as a previous target. */
int lb_target_name(struct link_bus *b, uint32_t node, char *out, size_t size) {
  if (!b || !out || size < 2) return -1;
  pw_thread_loop_lock(b->loop);
  int result = -1;
  for (struct lb_target *t = b->targets; t; t = t->next) {
    if (t->subject != node) continue;
    struct lb_object *target = NULL;
    uint64_t serial = lb_num(t->value);
    for (struct lb_object *o = b->objects; o && !target; o = o->next)
      if (o->kind == LB_NODE && ((serial && o->serial == serial) ||
                                 spa_streq(lb_prop(o, PW_KEY_NODE_NAME), t->value)))
        target = o;
    const char *name = target ? lb_prop(target, PW_KEY_NODE_NAME) : NULL;
    if (name && !lb_prop(target, "linx.link-in") && strlen(name) < size) {
      snprintf(out, size, "%s", name);
      result = 0;
    }
    break;
  }
  pw_thread_loop_unlock(b->loop);
  return result;
}

/* Return a stream to the named output if it still exists, else to the
 * desktop default. */
int lb_route_named(struct link_bus *b, uint32_t node, uint64_t serial, const char *name) {
  if (!b || !name) return -1;
  pw_thread_loop_lock(b->loop);
  int result = -1;
  struct lb_object *target = lb_node_named(b, name);
  if (lb_movable(b, node, serial)) {
    if (target && target->serial && !lb_prop(target, "linx.link-in")) {
      char value[32];
      snprintf(value, sizeof(value), "%" PRIu64, target->serial);
      result = pw_metadata_set_property(b->metadata, node, "target.object", "Spa:Id", value);
    } else {
      result = pw_metadata_set_property(b->metadata, node, "target.object", NULL, NULL);
    }
    if (result >= 0) result = lb_sync(b);
  }
  pw_thread_loop_unlock(b->loop);
  return result < 0 ? -1 : 0;
}

/* Set a Link In channel's send level: linear volume 0..1 (negative keeps
 * it) and mute (negative keeps it). PipeWire echoes the change as Props. */
int lb_set_level(struct link_bus *b, int channel, float volume, int mute) {
  if (!b || channel < 1 || channel > LB_CHANNELS) return -1;
  if (volume > 1.f || (volume >= 0.f && !isfinite(volume))) return -1;
  pw_thread_loop_lock(b->loop);
  int result = -1;
  struct lb_owned *x = &b->sinks[channel - 1];
  if (!b->broken && x->proxy && lb_owned_node(b, x)) {
    uint8_t buffer[512];
    struct spa_pod_builder pb = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
    struct spa_pod_frame frame;
    spa_pod_builder_push_object(&pb, &frame, SPA_TYPE_OBJECT_Props, SPA_PARAM_Props);
    if (volume >= 0.f) {
      float values[2] = {volume, volume};
      spa_pod_builder_add(&pb, SPA_PROP_channelVolumes,
                          SPA_POD_Array(sizeof(float), SPA_TYPE_Float, 2, values), 0);
    }
    if (mute >= 0) spa_pod_builder_add(&pb, SPA_PROP_mute, SPA_POD_Bool(mute != 0), 0);
    struct spa_pod *props = spa_pod_builder_pop(&pb, &frame);
    if (props && pw_node_set_param((struct pw_node *)x->proxy, SPA_PARAM_Props, 0, props) >= 0)
      result = 0;
  }
  pw_thread_loop_unlock(b->loop);
  return result;
}

/* Make `name` (an Audio/Sink node.name) the desktop's default output, as
 * `wpctl set-default` does: the session manager moves every stream that
 * follows the default. Streams LinX points at a channel stay there. */
int lb_set_default_output(struct link_bus *b, const char *name) {
  if (!b || !name || !*name || strlen(name) >= 200) return -1;
  pw_thread_loop_lock(b->loop);
  int result = -1;
  struct lb_object *target = lb_node_named(b, name);
  if (!b->broken && b->metadata && target &&
      spa_streq(lb_prop(target, PW_KEY_MEDIA_CLASS), "Audio/Sink")) {
    char quoted[512], value[600];
    if (spa_json_encode_string(quoted, sizeof(quoted), name) < (int)sizeof(quoted)) {
      snprintf(value, sizeof(value), "{ \"name\": %s }", quoted);
      result = pw_metadata_set_property(b->metadata, PW_ID_CORE, "default.configured.audio.sink",
                                        "Spa:String:JSON", value) < 0
                   ? -1
                   : 0;
    }
  }
  pw_thread_loop_unlock(b->loop);
  return result;
}
