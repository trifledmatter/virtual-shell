// yeah, imports and stuff
use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

// whatever, just a struct
pub struct CurlCommand;

impl Command for CurlCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // options, you know the drill
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
            // imports for wasm, whatever
            use futures::executor::block_on;
            use js_sys::Uint8Array;
            use uuid::Uuid;
            use wasm_bindgen::JsCast;
            use wasm_bindgen_futures::JsFuture;
            use web_sys::{Request, RequestInit, RequestMode, Response, Headers};

            // set up the request, blah blah
            let mut opts = RequestInit::new();
            opts.set_method(&method);
            opts.set_mode(RequestMode::Cors);
            let headers = Headers::new().unwrap();
            if let Some(ua) = &user_agent {
                headers.set("User-Agent", ua).ok();
            }
            for h in &custom_headers {
                if let Some((k, v)) = h.split_once(':') {
                    headers.set(k.trim(), v.trim()).ok();
                }
            }
            opts.set_headers(&headers);
            // clone file path before any block_on to avoid aliasing ctx
            let file_path = {
                // try to get filename, or just make one up
                let filename = output_file.clone().or_else(|| {
                    // we can't get content-disposition header until after fetch, so just use a temp name for now
                    None
                }).unwrap_or_else(|| format!("curl-{}.bin", Uuid::new_v4()));
                format!("{}/{}", ctx.cwd, filename)
            };
            let url_owned = url.clone();
            let request = match Request::new_with_str_and_init(&url_owned, &opts) {
                Ok(req) => req,
                Err(_) => return Err("[curl] Invalid URL".to_string()),
            };
            let window = web_sys::window().unwrap();
            let resp_value = match block_on(JsFuture::from(window.fetch_with_request(&request))) {
                Ok(val) => val,
                Err(_) => return Err(format!(
                    "[curl] Network error or host unreachable\n[curl] note: most public sites block browser requests due to CORS, so this is probably not your fault. try a CORS-friendly test endpoint like https://httpbin.org/get"
                )),
            };
            let resp: Response = resp_value.dyn_into().unwrap();
            if !resp.ok() {
                return Err(format!("[curl] HTTP error: {}", resp.status()));
            }
            // show headers if requested, i guess
            let mut header_str = String::new();
            if show_headers {
                let headers = resp.headers();
                let mut iter = js_sys::try_iter(&headers).unwrap();
                if let Some(iter) = iter {
                    for entry in iter {
                        if let Ok(arr) = entry {
                            let arr = js_sys::Array::from(&arr);
                            if arr.length() == 2 {
                                let k = arr.get(0).as_string().unwrap_or_default();
                                let v = arr.get(1).as_string().unwrap_or_default();
                                header_str.push_str(&format!("{}: {}\n", k, v));
                            }
                        }
                    }
                }
            }
            // get filename from content-disposition if possible (after fetch)
            let filename = output_file.clone().or_else(|| {
                resp.headers().get("content-disposition").ok().flatten()
                    .and_then(|cd| {
                        cd.split(';').find_map(|part| {
                            let part = part.trim();
                            if part.starts_with("filename=") {
                                Some(part.trim_start_matches("filename=").trim_matches('"').to_string())
                            } else { None }
                        })
                    })
            }).unwrap_or_else(|| format!("curl-{}.bin", Uuid::new_v4()));
            let file_path = format!("{}/{}", ctx.cwd, filename);
            // get bytes (unless HEAD), whatever
            let mut file_written = false;
            if method != "HEAD" {
                let buffer_promise = resp.array_buffer().unwrap();
                let buffer = match block_on(JsFuture::from(buffer_promise)) {
                    Ok(buf) => buf,
                    Err(_) => return Err("[curl] Failed to read response body".to_string()),
                };
                let array = Uint8Array::new(&buffer);
                let mut bytes = vec![0; array.length() as usize];
                array.copy_to(&mut bytes[..]);
                // save to vfs, i guess
                match ctx.vfs.create_file(&file_path, bytes) {
                    Ok(_) => file_written = true,
                    Err(e) => return Err(format!("[curl] Failed to save file: {}", e)),
                }
            }
            // output, whatever
            if silent {
                return Ok(String::new());
            }
            let mut result = String::new();
            if show_headers {
                result.push_str(&header_str);
            }
            if method == "HEAD" {
                // no file written, just show headers
                return Ok(result);
            }
            if file_written {
                result.push_str(&format!("[curl] Downloaded and saved as {}\n", filename));
            }
            Ok(result)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // not gonna work here, sorry
            Ok("[curl] This command only works in the browser (WASM)".to_string())
        }
    }
}
