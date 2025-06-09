// yeah, imports and stuff
use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

// whatever, just a struct
pub struct PingCommand;

impl Command for PingCommand {
    fn execute(&self, args: &[String], _ctx: &mut TerminalContext) -> CommandResult {
        // options, you know the drill
        let mut count = 4;
        let mut interval = 1000.0; // ms, i guess
        let mut deadline = None;
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
                "-i" => {
                    if let Some(val) = args.get(i+1) {
                        interval = val.parse::<f64>().unwrap_or(1.0) * 1000.0;
                        i += 1;
                    }
                }
                "-w" => {
                    if let Some(val) = args.get(i+1) {
                        deadline = val.parse::<f64>().ok().map(|d| d * 1000.0);
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
            // imports for wasm, whatever
            use wasm_bindgen::JsCast;
            use wasm_bindgen_futures::JsFuture;
            use web_sys::{Request, RequestInit, RequestMode, Response, window};
            use js_sys::Date;
            use gloo_timers::future::TimeoutFuture;
            use futures::executor::block_on;
            use std::time::Instant;

            // not really using this, but whatever
            let _results: Vec<()> = Vec::new();
            let start_time = Instant::now();
            let mut sent = 0;
            let mut received = 0;
            let mut total_rtt = 0.0;
            let mut min_rtt = f64::MAX;
            let mut max_rtt = 0.0;
            let mut output = String::new();
            // clone url so we don't borrow it across await/block_on
            for _seq in 0..count {
                let url_owned = url.clone();
                if let Some(deadline_ms) = deadline {
                    if start_time.elapsed().as_millis() as f64 > deadline_ms {
                        break;
                    }
                }
                // set up the request, blah blah
                let mut opts = RequestInit::new();
                opts.method("HEAD");
                opts.mode(RequestMode::Cors);
                let request = match Request::new_with_str_and_init(&url_owned, &opts) {
                    Ok(req) => req,
                    Err(_) => {
                        if !quiet { output.push_str("[ping] Invalid URL\n"); }
                        break;
                    }
                };
                let win = window().unwrap();
                let start = Date::now();
                let resp_value = block_on(JsFuture::from(win.fetch_with_request(&request)));
                let end = Date::now();
                sent += 1;
                match resp_value {
                    Ok(val) => {
                        let resp: Response = val.dyn_into().unwrap();
                        let status = resp.status();
                        let rtt = end - start;
                        if status >= 200 && status < 400 {
                            received += 1;
                            total_rtt += rtt;
                            if rtt < min_rtt { min_rtt = rtt; }
                            if rtt > max_rtt { max_rtt = rtt; }
                            if !quiet {
                                output.push_str(&format!("[ping] {}: reply, status {}, time {:.2} ms\n", url_owned, status, rtt));
                            }
                        } else {
                            if !quiet {
                                output.push_str(&format!("[ping] {}: no reply, status {}\n", url_owned, status));
                            }
                        }
                    }
                    Err(_) => {
                        if !quiet {
                            output.push_str(&format!(
                                "[ping] {}: network error or host unreachable\n[ping] note: most public sites block browser requests due to CORS, so this is probably not your fault. try a CORS-friendly test endpoint like https://httpbin.org/get\n",
                                url_owned
                            ));
                        }
                    }
                }
                if count > 1 {
                    block_on(TimeoutFuture::new(interval as u32));
                }
            }
            // print stats, because why not
            if !quiet {
                output.push_str(&format!(
                    "\n--- {} ping statistics ---\n{} packets transmitted, {} received, {:.1}% packet loss\n",
                    url.as_str(), sent, received, 100.0 * (sent - received) as f64 / sent as f64
                ));
                if received > 0 {
                    output.push_str(&format!(
                        "rtt min/avg/max = {:.2}/{:.2}/{:.2} ms\n",
                        min_rtt, total_rtt / received as f64, max_rtt
                    ));
                }
            }
            Ok(output)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // not gonna work here, sorry
            Ok("[ping] This command only works in the browser (WASM)".to_string())
        }
    }
}
