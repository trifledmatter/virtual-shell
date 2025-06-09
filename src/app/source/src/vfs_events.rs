use wasm_bindgen::prelude::*;
use serde_json;
use web_sys::{window, CustomEvent, CustomEventInit};

// yeah, we emit vfs events so the frontend can pretend to persist things
pub fn emit_vfs_event(event_type: &str, path: &str, content: Option<&[u8]>) {
    web_sys::console::log_4(
        &"[rust vfs] sending event:".into(),
        &event_type.into(),
        &"for path:".into(),
        &path.into(),
    );
    
    // grab window and document, or don't, whatever
    let win = window();
    let doc = win.as_ref().and_then(|w| w.document());
    
    if win.is_none() {
        web_sys::console::warn_1(&"[rust vfs] no window object, great".into());
        return;
    }

    // try the global callback first because dom events are unreliable garbage
    if let Some(win) = &win {
        let global = win.as_ref();
        
        // see if someone actually bothered to set up the callback
        if let Ok(callback_prop) = js_sys::Reflect::get(global, &"__vfsCallback".into()) {
            if !callback_prop.is_undefined() && callback_prop.is_function() {
                web_sys::console::log_1(&"[rust vfs] found callback, actually calling it".into());
                
                let callback = callback_prop.dyn_into::<js_sys::Function>().unwrap();
                
                // throw some data together
                let mut event_data = serde_json::json!({
                    "path": path
                });
                
                if let Some(content_bytes) = content {
                    event_data["content"] = serde_json::json!(content_bytes);
                }
                
                let data_js = serde_wasm_bindgen::to_value(&event_data).unwrap_or(JsValue::NULL);
                
                // fingers crossed this doesn't explode
                match callback.call2(&JsValue::NULL, &event_type.into(), &data_js) {
                    Ok(_) => {
                        web_sys::console::log_1(&"[rust vfs] callback worked, shocking".into());
                        return; // bail early, we're done here
                    }
                    Err(e) => {
                        web_sys::console::error_2(
                            &"[rust vfs] callback failed, as expected:".into(),
                            &e,
                        );
                    }
                }
            } else {
                web_sys::console::warn_1(&"[rust vfs] callback exists but isn't a function, nice job".into());
            }
        } else {
            web_sys::console::warn_1(&"[rust vfs] no callback found, falling back to dom event hell".into());
        }
    }
    
    let mut event_detail = serde_json::json!({
        "path": path
    });
    
    // slap content in there for writes
    if let Some(content_bytes) = content {
        event_detail["content"] = serde_json::json!(content_bytes);
        web_sys::console::log_3(
            &"[rust vfs] content size:".into(),
            &(content_bytes.len() as u32).into(),
            &"bytes".into(),
        );
    }
    
    // make a fancy custom event
    let mut event_init = CustomEventInit::new();
    event_init.set_bubbles(true); // bubble up because why not
    event_init.set_cancelable(true);
    event_init.set_detail(&serde_wasm_bindgen::to_value(&event_detail).unwrap_or(JsValue::NULL));
    
    match CustomEvent::new_with_event_init_dict(event_type, &event_init) {
        Ok(custom_event) => {
            let mut dispatched = false;
            
            // try window first
            if let Some(win) = &win {
                match win.dispatch_event(&custom_event) {
                    Ok(_) => {
                        web_sys::console::log_2(
                            &"[rust vfs] event sent to window:".into(),
                            &event_type.into(),
                        );
                        dispatched = true;
                    }
                    Err(e) => {
                        web_sys::console::error_3(
                            &"[rust vfs] window dispatch failed:".into(),
                            &event_type.into(),
                            &e,
                        );
                    }
                }
            }
            
            // also try document because redundancy is fun
            if let Some(doc) = &doc {
                match doc.dispatch_event(&custom_event) {
                    Ok(_) => {
                        web_sys::console::log_2(
                            &"[rust vfs] event sent to document:".into(),
                            &event_type.into(),
                        );
                        dispatched = true;
                    }
                    Err(e) => {
                        web_sys::console::error_3(
                            &"[rust vfs] document dispatch failed:".into(),
                            &event_type.into(),
                            &e,
                        );
                    }
                }
            }
            
            if !dispatched {
                web_sys::console::error_1(&"[rust vfs] couldn't dispatch anywhere, good luck".into());
            }
        }
        Err(e) => {
            web_sys::console::error_3(
                &"[rust vfs] couldn't even create the event:".into(),
                &event_type.into(),
                &e,
            );
        }
    }
} 