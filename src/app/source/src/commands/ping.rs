use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct PingCommand;

impl Command for PingCommand {
    fn execute(&self, args: &[String], _ctx: &mut TerminalContext) -> CommandResult {
        // parse args like we always do
        let mut count = 4;
        let mut quiet = false;
        let mut url = None;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-c" => {
                    if let Some(val) = args.get(i+1) {
                        count = val.parse().unwrap_or(4);
                        i += 1;
                    }
                }
                "-q" => {
                    quiet = true;
                }
                arg if !arg.starts_with('-') && url.is_none() => {
                    url = Some(arg.to_string());
                }
                _ => {}
            }
            i += 1;
        }
        let url = match url {
            Some(u) => u,
            None => return Err("Usage: ping [options] <url>".to_string()),
        };
        
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::{spawn_local, JsFuture};
            use wasm_bindgen::JsCast;
            use web_sys::{Request, RequestInit, RequestMode, Response, window};
            use js_sys::Date;

            // check if url is remotely valid
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }

            let url_clone = url.clone();
            let quiet_clone = quiet;
            
            // spawn async task because blocking is for noobs
            spawn_local(async move {
                let mut sent = 0;
                let mut received = 0;
                let mut total_rtt = 0.0;
                let mut min_rtt = f64::MAX;
                let mut max_rtt = 0.0;
                
                let window = match window() {
                    Some(w) => w,
                    None => {
                        crate::send_async_result("No window object available");
                        return;
                    }
                };

                for seq in 0..count {
                    sent += 1;
                    let start_time = Date::now();
                    
                    // head request to avoid cors drama
                    let mut opts = RequestInit::new();
                    opts.set_method("HEAD");
                    opts.set_mode(RequestMode::NoCors);
                    
                    let request = match Request::new_with_str_and_init(&url_clone, &opts) {
                        Ok(req) => req,
                        Err(_) => {
                            if !quiet_clone {
                                crate::send_async_result(&format!("{}: Invalid URL", url_clone));
                            }
                            continue;
                        }
                    };
                    
                    // await the fetch like civilized people
                    match JsFuture::from(window.fetch_with_request(&request)).await {
                        Ok(response_val) => {
                            let end_time = Date::now();
                            let rtt = end_time - start_time;
                            
                            if let Ok(response) = response_val.dyn_into::<Response>() {
                                let status = response.status();
                                if response.ok() {
                                    received += 1;
                                    total_rtt += rtt;
                                    if rtt < min_rtt { min_rtt = rtt; }
                                    if rtt > max_rtt { max_rtt = rtt; }
                                    if !quiet_clone {
                                        crate::send_async_result(&format!("{}: reply, time {:.2} ms, seq={}", url_clone, rtt, seq));
                                    }
                                } else {
                                    if !quiet_clone {
                                        crate::send_async_result(&format!("{}: no reply, status {}, seq={}", url_clone, status, seq));
                                    }
                                }
                            } else {
                                // probably cors, because internet
                                if !quiet_clone {
                                    crate::send_async_result(&format!("{}: request blocked (CORS), seq={}", url_clone, seq));
                                }
                            }
                        }
                        Err(_) => {
                            if !quiet_clone {
                                crate::send_async_result(&format!("{}: network error or CORS restriction, seq={}", url_clone, seq));
                            }
                        }
                    }
                    
                    // wait between pings like ping does
                    if seq < count - 1 {
                        gloo_timers::future::TimeoutFuture::new(1000).await;
                    }
                }
                
                // show stats if not quiet
                if !quiet_clone {
                    crate::send_async_result(&format!(
                        "\n--- {} ping statistics ---\n{} packets transmitted, {} received, {:.1}% packet loss",
                        url_clone, sent, received, if sent > 0 { 100.0 * (sent - received) as f64 / sent as f64 } else { 0.0 }
                    ));
                    
                    if received > 0 {
                        crate::send_async_result(&format!(
                            "rtt min/avg/max = {:.2}/{:.2}/{:.2} ms",
                            min_rtt, total_rtt / received as f64, max_rtt
                        ));
                    }
                }
            });
            
            // return immediately with helpful info
            Ok(format!("Starting ping to {} ({} packets)...\n💡 NOTE: If you get CORS errors, try these working endpoints:\n  • https://httpbin.org/get\n  • https://jsonplaceholder.typicode.com/posts/1", url, count))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok("This command only works in the browser (WASM)".to_string())
        }
    }
}
