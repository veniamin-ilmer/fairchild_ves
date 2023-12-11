#![forbid(unsafe_code)]

mod sound;
mod video;
mod keyboard;
mod side_panel;

use wasm_bindgen::prelude::*;

use boards::fairchild_ves;

#[wasm_bindgen]
pub async fn run() {
  std::panic::set_hook(Box::new(console_error_panic_hook::hook)); //Panics appear more descriptive in the browser console.
  
  let window = web_sys::window().unwrap();
  let location = window.location();
  let search = location.search().unwrap();
  let params = parse_query_string(&search);
  
  let document = window.document().unwrap();
  
  let bios = if let Some(link) = params.get("bios") {
    if let Some(element) = document.get_element_by_id("bios") {
      if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        input.set_value(link);
      }
    }
    fetch(&window, link).await
  } else { None };

  let rom = if let Some(link) = params.get("rom") {
    if let Some(element) = document.get_element_by_id("rom") {
      if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        input.set_value(link);
      }
    }
    fetch(&window, link).await
  } else { None };
    
  let mut board = fairchild_ves::Board::new(bios, rom);
  let mut keyboard = keyboard::Keyboard::new();
  let mut video = video::Video::new();
  let mut sound = sound::Sound::new();
  let mut side_panel = side_panel::SidePanel::new();

  let mut refresh_count = 0;
  //Frame cycle
  loop {
    //Instruction cycle
    loop {
      keyboard.run_cycle(&mut board);
      let clock_ticks = board.run_cycle() as usize;
      board.roms[0].print();
      board.cpu.print();
      if sound.run_cycle(&board, clock_ticks) {
        //We filled up the sound buffer. We shouldn't run any more instructions until we get a new buffer. Go to the next frame.
        break;
      }
    }

    refresh_count += 1;
    video.run_refresh_cycle(&board);

    if refresh_count % 2 == 0 {
      keyboard.run_refresh_cycle(&mut board);
      side_panel.print_memory(&board);
      refresh_count = 0;
    }
    
    sound.run_refresh_cycle().await;  //Wait till the previous sound was played.
  }

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
  if let Ok(request) = web_sys::Request::new_with_str(link) {
    if let Ok(resp_value) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
      if let Ok(resp) = resp_value.dyn_into::<web_sys::Response>() {
        if let Ok(buffer_future) = resp.array_buffer() {
          if let Ok(buffer) = wasm_bindgen_futures::JsFuture::from(buffer_future).await {
            let uint8_array = js_sys::Uint8Array::new(&buffer);
            return Some(uint8_array.to_vec());
          }
        }
      }
    }
  }
  None
}