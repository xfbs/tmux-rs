use foo::{
    bufferevent, bufferevent_disable, bufferevent_enable, bufferevent_free, bufferevent_get_output, bufferevent_new,
    bufferevent_setwatermark, bufferevent_write, bufferevent_write_buffer, evbuffer, evbuffer_add, evbuffer_add_printf,
    evbuffer_add_vprintf, evbuffer_drain, evbuffer_eol_style, evbuffer_eol_style_EVBUFFER_EOL_LF, evbuffer_free,
    evbuffer_get_length, evbuffer_new, evbuffer_pullup, evbuffer_readln, event, event_active, event_add, event_base,
    event_del, event_get_method, event_get_version, event_initialized, event_loop, event_once, event_pending,
    event_reinit, event_set, event_set_log_callback, timeval, EVLOOP_ONCE, EV_PERSIST, EV_READ, EV_SIGNAL, EV_TIMEOUT,
    EV_WRITE, SIZE_MAX,
};
