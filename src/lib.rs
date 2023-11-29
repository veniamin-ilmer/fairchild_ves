#![forbid(unsafe_code)]

mod sound;      //100 frames per second
mod video;      //50 frames per second
mod keyboard;   //25 frames per second
mod side_panel; //12.5 frames per second

/// 100 frames per second is 10 milliseconds. cpu runs at 500 ns per clock. So 10 millisecond / 500 ns gives us how many clocks per frame.
const CLOCKS_PER_REFRESH: usize = 10_000_000 / 500;

use wasm_bindgen::prelude::*;

use boards::fairchild_ves;

#[wasm_bindgen]
pub async fn run() {
  std::panic::set_hook(Box::new(console_error_panic_hook::hook)); //Panics appear more descriptive in the browser console.
  
  let window = web_sys::window().unwrap();
  let location = window.location();
  let search = location.search().unwrap();
  let params = parse_query_string(&search);
  
  let bios = if let Some(link) = params.get("bios") {
    let data1 = fetch(&window, link).await;
    if let Some(data) = data1 {
      panic!("len: {}", data.len());
    }
    data1
  } else { None };

  let rom = if let Some(link) = params.get("rom") {
    fetch(&window, link).await
  } else { None };

  
  let mut board = fairchild_ves::Board::new(bios, rom);
  let mut keyboard = keyboard::Keyboard::new();
  let mut video = video::Video::new();
  let mut sound = sound::Sound::new();
  let mut side_panel = side_panel::SidePanel::new();

  
  let mut refresh_count = 0;
  loop {
    let start_time = instant::Instant::now();

    refresh_count += 1;
    sound.run_refresh_cycle(&board);
    
    if refresh_count % 2 == 0 {
      video.run_refresh_cycle(&board);
    }
    
    if refresh_count % 4 == 0 {
      keyboard.run_refresh_cycle(&mut board);
    }
    
    if refresh_count % 8 == 0 {
      side_panel.print_memory(&board);
      refresh_count = 0;
    }
    
    let mut clock_ticks = 0;
    while CLOCKS_PER_REFRESH > clock_ticks {
      keyboard.run_cycle(&mut board);
      sound.run_cycle(&board);
      clock_ticks += board.run_cycle() as usize;
    }
    
    //Getting the current time is actually an expensive operation in web browsers, so I only do it in the refresh cycle.
    let duration = instant::Instant::now() - start_time;
    sleep(&window, 10 - duration.as_millis() as i32).await;  //40 milliseconds delay = 25 frames per second
  }

}

/// A trick to get browsers to "sleep" by awaiting a set_timeout
async fn sleep(window: &web_sys::Window, ms: i32) {
  let promise = js_sys::Promise::new(&mut |resolve, _| {
    window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms).unwrap();
  });
  let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}


fn parse_query_string(query_string: &str) -> std::collections::HashMap<String, String> {
  let mut result = std::collections::HashMap::new();

  for pair in query_string.trim_start_matches('?').split('&') {
    let mut key_value = pair.split('=');
    if let (Some(key), Some(value)) = (key_value.next(), key_value.next()) {
      result.insert(key.to_string(), value.to_string());
    }
  }

  result
}

async fn fetch(window: &web_sys::Window, link: &str) -> Option<Vec<u8>> {
  let mut opts = web_sys::RequestInit::new();
  opts.method("GET");
  opts.mode(web_sys::RequestMode::Cors);

  if let Ok(request) = web_sys::Request::new_with_str_and_init(link, &opts) {
    if let Ok(resp_value) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
      if let Ok(resp) = resp_value.dyn_into::<web_sys::Response>() {
        // Read the response body as an ArrayBuffer
        if let Ok(body) = resp.array_buffer() {

          // Convert ArrayBuffer to Vec<u8>
          let uint8_array = js_sys::Uint8Array::new(&body);
          return Some(uint8_array.to_vec());
        } else {
        panic!("fail1");
        }
      } else {
      panic!("fail2");
      }
    } else {
    panic!("fail3");
    }
  } else {
    panic!("fail4");
  }
  None
}