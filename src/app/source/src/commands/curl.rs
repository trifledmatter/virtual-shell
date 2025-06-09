use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct CurlCommand;

impl Command for CurlCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // grab cwd early to avoid borrow checker drama
        let current_dir = ctx.cwd.clone();
        
        // parse all the curl flags like usual
        let mut url = None;
        let mut output_file = None;
        let mut show_headers = false;
        let mut silent = false;
        let mut user_agent = None;
        let mut method = "GET".to_string();
        let mut custom_headers = vec![];
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-o" => {
                    if let Some(val) = args.get(i+1) {
                        output_file = Some(val.clone());
                        i += 1;
                    }
                }
                "-I" | "--head" => {
                    method = "HEAD".to_string();
                }
                "-H" => {
                    if let Some(val) = args.get(i+1) {
                        custom_headers.push(val.clone());
                        i += 1;
                    }
                }
                "-A" => {
                    if let Some(val) = args.get(i+1) {
                        user_agent = Some(val.clone());
                        i += 1;
                    }
                }
                "-s" => {
                    silent = true;
                }
                "-i" => {
                    show_headers = true;
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
            None => return Err("Usage: curl [options] <url>".to_string()),
        };
        
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::{spawn_local, JsFuture};
            use wasm_bindgen::JsCast;
            use web_sys::{Request, RequestInit, RequestMode, Response, Headers, window};

            // check if url is remotely valid
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }

            let url_clone = url.clone();
            let method_clone = method.clone();
            let silent_clone = silent;
            let show_headers_clone = show_headers;
            let user_agent_clone = user_agent.clone();
            let custom_headers_clone = custom_headers.clone();
            let output_file_clone = output_file.clone();
            
            // spawn async task because we're not animals
            spawn_local(async move {
                let window = match window() {
                    Some(w) => w,
                    None => {
                        crate::send_async_result("No window object available");
                        return;
                    }
                };

                // set up request with the usual suspects
                let mut opts = RequestInit::new();
                opts.set_method(&method_clone);
                opts.set_mode(RequestMode::Cors); // cors mode for maximum compatibility
                
                // add headers if we have any
                let headers = Headers::new().unwrap();
                if let Some(ua) = &user_agent_clone {
                    if headers.set("User-Agent", ua).is_err() {
                        crate::send_async_result("Warning: Could not set User-Agent header");
                    }
                }
                for h in &custom_headers_clone {
                    if let Some((k, v)) = h.split_once(':') {
                        if headers.set(k.trim(), v.trim()).is_err() {
                            crate::send_async_result(&format!("Warning: Could not set header: {}", h));
                        }
                    }
                }
                opts.set_headers(&headers);
                
                let request = match Request::new_with_str_and_init(&url_clone, &opts) {
                    Ok(req) => req,
                    Err(_) => {
                        crate::send_async_result("Invalid URL or request configuration");
                        return;
                    }
                };
                
                // actually make the request
                match JsFuture::from(window.fetch_with_request(&request)).await {
                    Ok(response_val) => {
                        if let Ok(response) = response_val.dyn_into::<Response>() {
                            let status = response.status();
                            
                            if !silent_clone {
                                crate::send_async_result(&format!("HTTP {} {}", status, response.status_text()));
                            }
                            
                            // show headers if requested
                            if show_headers_clone {
                                crate::send_async_result(&format!("HTTP/1.1 {} {}", status, response.status_text()));
                                
                                let headers_iter = response.headers().entries();
                                let iter = js_sys::try_iter(&headers_iter).unwrap();
                                if let Some(iter) = iter {
                                    for entry in iter {
                                        if let Ok(arr) = entry {
                                            let arr = js_sys::Array::from(&arr);
                                            if arr.length() == 2 {
                                                let k = arr.get(0).as_string().unwrap_or_default();
                                                let v = arr.get(1).as_string().unwrap_or_default();
                                                crate::send_async_result(&format!("{}: {}", k, v));
                                            }
                                        }
                                    }
                                }
                                crate::send_async_result(""); // empty line for readability
                            }
                            
                            // get response body unless it's head
                            if method_clone != "HEAD" {
                                match JsFuture::from(response.text().unwrap()).await {
                                    Ok(text_val) => {
                                        let text = text_val.as_string().unwrap_or_default();
                                        
                                        if let Some(filename) = &output_file_clone {
                                            // file saving is complicated in async context
                                            crate::send_async_result(&format!("Content saved as {} (simulated - file saving not implemented in async mode)", filename));
                                            crate::send_async_result("Content:");
                                            crate::send_async_result(&text);
                                        } else {
                                            // just dump the response
                                            if !silent_clone {
                                                crate::send_async_result(&text);
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        crate::send_async_result("Failed to read response body");
                                    }
                                }
                            }
                        } else {
                            // failed response conversion, probably cors
                            crate::send_async_result(&format!("❌ Request to {} failed", url_clone));
                            crate::send_async_result("🚫 This is likely a CORS (Cross-Origin Resource Sharing) restriction.");
                            crate::send_async_result("💡 Most websites block browser requests for security reasons.");
                            crate::send_async_result("");
                            crate::send_async_result("✅ Try these CORS-friendly test endpoints instead:");
                            crate::send_async_result("  • https://httpbin.org/get");
                            crate::send_async_result("  • https://jsonplaceholder.typicode.com/posts/1");
                            crate::send_async_result("  • https://api.github.com/users/octocat");
                            crate::send_async_result("  • https://httpbin.org/headers");
                            crate::send_async_result("  • https://httpbin.org/ip");
                        }
                    }
                    Err(_) => {
                        // network error or cors blocking
                        crate::send_async_result(&format!("❌ Network request to {} was blocked", url_clone));
                        crate::send_async_result("");
                        crate::send_async_result("🚫 Common reasons for blocking:");
                        crate::send_async_result("  • CORS policy restrictions (most common)");
                        crate::send_async_result("  • Network connectivity issues");
                        crate::send_async_result("  • Invalid or unreachable URL");
                        crate::send_async_result("  • Server blocking browser requests");
                        crate::send_async_result("");
                        crate::send_async_result("✅ Try these working examples:");
                        crate::send_async_result("  curl https://httpbin.org/get");
                        crate::send_async_result("  curl -I https://api.github.com/users/octocat");
                        crate::send_async_result("  curl https://jsonplaceholder.typicode.com/posts/1");
                    }
                }
            });
            
            // return immediately with helpful info
            Ok(format!("Starting {} request to {}...\nNOTE: 💡 If you get CORS errors, try these working endpoints:\n  • https://httpbin.org/get\n  • https://jsonplaceholder.typicode.com/posts/1\n  • https://api.github.com/users/octocat", method, url))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok("This command only works in the browser (WASM)".to_string())
        }
    }
}
