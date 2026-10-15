/* Aggregate-only observer of the retained desktop sink. No audio is saved or
 * handed to Rust/UI. All callbacks and destruction belong to this core. */
static void dt_meter_invalidate(struct desktop_meter *m) {
  if (m->mailbox) atomic_store(&m->mailbox->valid,0);
  m->window_frames=0;
  for (int i=0;i<2;i++) { m->window_peak[i]=0.f;m->window_power[i]=0.; }
}
static bool dt_meter_reduce(const float *left, const float *right, uint32_t n,
                            float peak[2], float power[2]) {
  if (!left || !right || !n || n>8192) return false;
  const float *input[2]={left,right};
  for (int side=0;side<2;side++) {
    double sum=0.;float maximum=0.f;
    for (uint32_t i=0;i<n;i++) {
      float v=input[side][i];
      if (!isfinite(v)) return false;
      float absolute=v<0.f?-v:v;
      if (absolute>maximum) maximum=absolute;
      sum+=(double)v*(double)v;
    }
    double mean=sum/n;
    if (mean>FLT_MAX || !isfinite(mean)) return false;
    peak[side]=maximum;power[side]=(float)mean;
  }
  return true;
}
static void dt_meter_process(void *data, struct spa_io_position *position) {
  struct desktop_meter *m=data;
  if (!m->mailbox || !position || !position->clock.duration ||
      position->clock.duration>8192) { dt_meter_invalidate(m);return; }
  uint32_t n=(uint32_t)position->clock.duration;
  const float *left=pw_filter_get_dsp_buffer(m->ports[0],n);
  const float *right=pw_filter_get_dsp_buffer(m->ports[1],n);
  float peak[2],power[2];
  if (!dt_meter_reduce(left,right,n,peak,power) || UINT64_MAX-m->frames<n) {
    dt_meter_invalidate(m);return;
  }
  m->frames+=n;
  /* A roughly 40-ms window retains every block's peak between UI reads and
   * averages power by sample count. Never enqueue or retain audio buffers. */
  uint32_t rate=position->clock.rate.num ?
      position->clock.rate.denom/position->clock.rate.num : 0;
  if (rate<8000 || rate>384000) { dt_meter_invalidate(m);return; }
  m->window_frames+=n;
  for (int i=0;i<2;i++) {
    if (peak[i]>m->window_peak[i]) m->window_peak[i]=peak[i];
    m->window_power[i]+=(double)power[i]*n;
  }
  if (m->window_frames<rate/25) return;
  atomic_fetch_add(&m->mailbox->sequence,1);
  for (int i=0;i<2;i++) {
    peak[i]=m->window_peak[i];power[i]=(float)(m->window_power[i]/m->window_frames);
    uint32_t p,q;memcpy(&p,&peak[i],4);memcpy(&q,&power[i],4);
    atomic_store(&m->mailbox->peak[i],p);
    atomic_store(&m->mailbox->power[i],q);
    m->window_peak[i]=0.f;m->window_power[i]=0.;
  }
  m->window_frames=0;
  atomic_store(&m->mailbox->frames,m->frames);
  atomic_fetch_add(&m->mailbox->sequence,1);
  atomic_store(&m->mailbox->valid,1);
}
static const struct pw_filter_events dt_meter_events={
  .version=PW_VERSION_FILTER_EVENTS,.process=dt_meter_process
};
static bool dt_meter_graph(struct session *s) {
  struct desktop_target *d=s->desktop;struct desktop_meter *m=&d->meter;
  if (!m->filter) return !m->ready && !m->node.serial;
  struct object *node=find(s,m->node.id,NODE), *inputs[2],*outputs[2];
  if (!m->ready || pw_filter_get_node_id(m->filter)!=m->node.id ||
      pw_filter_get_state(m->filter,NULL)==PW_FILTER_STATE_ERROR ||
      !dt_ref(node,m->node) || !spa_streq(prop(node,OWNER),d->owner) ||
      !dt_ports(s,m->node,PW_DIRECTION_INPUT,inputs) ||
      !dt_ports_mode(s,d->sink,PW_DIRECTION_OUTPUT,outputs,true)) return false;
  unsigned ports=0,links=0;
  for (size_t i=0;i<s->count;i++) {
    struct object *o=s->objects[i];if (!o->present) continue;
    if (o->kind==PORT && num(prop(o,PW_KEY_NODE_ID))==m->node.id) {
      if (o->direction!=PW_DIRECTION_INPUT) return false;
      ports++;
    }
    if (o->kind!=LINK || (o->in_node!=m->node.id && o->out_node!=m->node.id)) continue;
    bool expected=false;
    for (int side=0;side<2;side++) {
      struct created *c=m->links[side];
      if (!dt_ref(inputs[side],m->inputs[side])) return false;
      if (c && c->proxy && c->bound && c->info && !c->removed && !c->error &&
          c->id==o->id && c->serial==serial(o) && o->info &&
          o->state==PW_LINK_STATE_ACTIVE && o->out_node==d->sink.id &&
          o->out_port==outputs[side]->id && o->in_node==m->node.id &&
          o->in_port==inputs[side]->id && c->out_port==o->out_port && c->in_port==o->in_port)
        expected=true;
    }
    if (!expected) return false;
    links++;
  }
  return ports==2 && links==2;
}
int bcl_desktop_meter_start(struct session *s, struct meter_mailbox *mailbox,
                            struct receipt *node_result, struct receipt *links_result) {
  if (!s || !s->desktop || !mailbox || !node_result || !links_result ||
      bcl_desktop_bridge_check(s)<0) return -1;
  struct desktop_target *d=s->desktop;struct desktop_meter *m=&d->meter;
  if (m->filter || m->node.serial || m->mailbox) return -1;
  m->mailbox=mailbox;dt_meter_invalidate(m);
  char name[160];snprintf(name,sizeof(name),"%s-meter",d->owner);
  m->filter=pw_filter_new(s->core,"LinX level meter",
      pw_properties_new(PW_KEY_NODE_NAME,name,PW_KEY_NODE_DESCRIPTION,"LinX stereo levels",
          PW_KEY_MEDIA_CLASS,"Audio/Filter",PW_KEY_MEDIA_TYPE,"Audio",
          PW_KEY_MEDIA_CATEGORY,"Monitor",PW_KEY_MEDIA_ROLE,"DSP",
          PW_KEY_NODE_AUTOCONNECT,"false",PW_KEY_OBJECT_LINGER,"false",
          "node.passive","true","node.virtual","true","state.restore-props","false",
          "state.restore-target","false",OWNER,d->owner,NULL));
  if (!m->filter) return -1;
  pw_filter_add_listener(m->filter,&m->listener,&dt_meter_events,m);
  const char *channels[2]={"FL","FR"};
  for (int i=0;i<2;i++) {
    m->ports[i]=pw_filter_add_port(m->filter,PW_DIRECTION_INPUT,
        PW_FILTER_PORT_FLAG_MAP_BUFFERS,1,
        pw_properties_new(PW_KEY_FORMAT_DSP,"32 bit float mono audio",
            PW_KEY_PORT_NAME,channels[i],PW_KEY_AUDIO_CHANNEL,channels[i],NULL),NULL,0);
    if (!m->ports[i]) return -1;
  }
  if (pw_filter_connect(m->filter,PW_FILTER_FLAG_RT_PROCESS,NULL,0)<0) return -1;
  int64_t deadline=millis()+3000;
  struct object *node=NULL,*inputs[2],*outputs[2];
  do {
    if (barrier(s)<0 || barrier(s)<0) return -1;
    uint32_t id=pw_filter_get_node_id(m->filter);
    node=id==SPA_ID_INVALID?NULL:find(s,id,NODE);
    if (node && node->info && serial(node) && dt_ports(s,(struct receipt){id,serial(node)},PW_DIRECTION_INPUT,inputs)) break;
    if (pw_filter_get_state(m->filter,NULL)==PW_FILTER_STATE_ERROR ||
        pw_loop_iterate(pw_main_loop_get_loop(s->loop),20)<0) return -1;
  } while (millis()<deadline);
  if (!node || !node->info || !serial(node) || !spa_streq(prop(node,OWNER),d->owner) ||
      !dt_ports(s,(struct receipt){node->id,serial(node)},PW_DIRECTION_INPUT,inputs) ||
      !dt_ports_mode(s,d->sink,PW_DIRECTION_OUTPUT,outputs,true)) return -1;
  m->node=(struct receipt){node->id,serial(node)};*node_result=m->node;
  for (int i=0;i<2;i++) {
    m->inputs[i]=(struct receipt){inputs[i]->id,serial(inputs[i])};
    struct pw_properties *p=pw_properties_new(PW_KEY_OBJECT_LINGER,"false",
        PW_KEY_LINK_PASSIVE,"true",OWNER,d->owner,NULL);
    if (!p) return -1;
    m->links[i]=playback_created(s,"link-factory",PW_TYPE_INTERFACE_Link,
        PW_VERSION_LINK,p,d->owner,false,outputs[i],inputs[i]);
    pw_properties_free(p);
    if (!m->links[i] || playback_ack(s,m->links[i],false,&links_result[i])<0) return -1;
  }
  m->ready=true;
  return dt_meter_graph(s) && dt_bridge_graph(s)?0:-1;
}
static void dt_meter_destroy(struct session *s) {
  struct desktop_meter *m=&s->desktop->meter;
  if (m->filter) {
    /* disconnect/destroy synchronizes the data-loop callback before the
     * caller may release the Rust mailbox Arc. Never destroy a registry ID. */
    spa_hook_remove(&m->listener);
    pw_filter_destroy(m->filter);m->filter=NULL;
  }
  dt_meter_invalidate(m);m->ready=false;
}
static int dt_meter_stop(struct session *s) {
  struct desktop_meter *m=&s->desktop->meter;
  for (int i=0;i<2;i++) {
    struct created *c=m->links[i];
    if (c && c->proxy) {
      spa_hook_remove(&c->listener);spa_hook_remove(&c->info_listener);
      pw_proxy_destroy(c->proxy);c->proxy=NULL;
    }
  }
  dt_meter_destroy(s);
  if (!m->mailbox) return 0;
  if (barrier(s)<0 || barrier(s)<0) return -1;
  if (m->node.serial && dt_receipt_present(s,m->node,NODE)) return -1;
  for (int i=0;i<2;i++) {
    struct created *c=m->links[i];
    if (c && dt_receipt_present(s,(struct receipt){c->id,c->serial},LINK)) return -1;
    if (m->inputs[i].serial && dt_receipt_present(s,m->inputs[i],PORT)) return -1;
  }
  return 0;
}
#ifdef BCL_TEST_FAULTS
int bcl_desktop_test_meter_math(void) {
  struct meter_mailbox mailbox={0};
  struct desktop_meter state={.mailbox=&mailbox,.window_frames=128,
      .window_peak={1.f,1.f},.window_power={128.,128.}};
  atomic_store(&mailbox.valid,1);
  dt_meter_invalidate(&state);
  if (atomic_load(&mailbox.valid) || state.window_frames || state.window_peak[0] ||
      state.window_peak[1] || state.window_power[0] || state.window_power[1]) return -1;
  const float left[]={0.f,.5f,-.5f,1.f},right[]={-.25f,.25f,-.25f,.25f};
  float peak[2],power[2];
  if (!dt_meter_reduce(left,right,4,peak,power) || peak[0]!=1.f ||
      peak[1]!=.25f || power[0]!=.375f || power[1]!=.0625f) return -1;
  const float zero[]={0.f,0.f},bad[]={NAN,INFINITY};
  if (!dt_meter_reduce(zero,zero,2,peak,power) || peak[0]!=0.f || power[1]!=0.f ||
      dt_meter_reduce(bad,zero,2,peak,power) || dt_meter_reduce(NULL,zero,2,peak,power) ||
      dt_meter_reduce(zero,zero,0,peak,power) || dt_meter_reduce(zero,zero,8193,peak,power)) return -1;
  return 0;
}
#endif
