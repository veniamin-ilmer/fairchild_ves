use boards::fairchild_ves;
use wasm_bindgen::prelude::*;

pub(super) struct Video {
  canvas_context: js_sys::Object,
  memory: [[((bool, bool),(bool,bool)); 64]; 128],  //(color, background). Each consists of two bits, which sets the color.
}

impl Video {
  pub fn new() -> Self {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let dummy = document.get_element_by_id("canvas").expect("the canvas is missing");
    let canvas: &web_sys::HtmlCanvasElement = dummy.dyn_ref().unwrap();
    let canvas_context = canvas.get_context("2d").expect("the canvas should have a context").expect("the canvas should have a context");
    //let canvas_context: &web_sys::CanvasRenderingContext2d = dummy.dyn_ref().unwrap();
    Self {
      canvas_context: canvas_context,
      memory: [[((false, false),(false, false)); 64]; 128],
    }
  }
  
  /// Out of the 128x64 ram, display only 102x58
  pub fn run_refresh_cycle(&mut self, board: &fairchild_ves::Board) {
    //Browsers are actually really inefficient at drawing pixels. So I hold in memory what the browser last drew and only draw what changed.
    for y in 2..=59 {
      
      let background = get_background_bits(board, y);
      let html_background = match background {
        (true, true) => "#94ffa4",
        (false, true) => "#cdd2ff",
        (true, false) => "#e6e2e6",
        (false, false) => "#000000",
      };

      for x in 20..=127 - 4 { //Lots of pixels are not displayed on the TV..
        let color = get_pixel(board, x, y);
        if self.memory[x][y] != (color, background) {
          self.memory[x][y] = (color, background);
          let html_color = wasm_bindgen::JsValue::from(match color {
            (true, true) => "#00ce5a",
            (false, true) => "#ff3052",
            (true, false) => "#4a3cf6",
            (false, false) => html_background,
          });
          let canvas_context: &web_sys::CanvasRenderingContext2d = self.canvas_context.dyn_ref().unwrap();
          canvas_context.set_fill_style(&html_color);
          canvas_context.fill_rect(((127 - 4 - x) * 5) as f64, ((59 - y) * 6) as f64, 5.0, 6.0);  //The TV stretches out the pixels changing 16:9 into 4:3. I get close to this when scaling.
        }
      }
    }
  }
}

/// Pixels 1 and 2 contain the background..
fn get_background_bits(board: &fairchild_ves::Board, y: usize) -> (bool, bool) {
  let address = y * 128 + 1;
  let bit0 = if address < 0x1000 {
    board.vram[2].read_bit(address)
  } else {
    board.vram[3].read_bit(address - 0x1000)
  };
  let bit1 = if address < 0x1000 {
    board.vram[2].read_bit(address + 1)
  } else {
    board.vram[3].read_bit((address + 1) - 0x1000)
  };
  (bit0, bit1)
}

fn get_pixel(board: &fairchild_ves::Board, x: usize, y: usize) -> (bool, bool) {
  let address = x + y * 128;
  let bit0 = if address < 0x1000 {
    board.vram[0].read_bit(address)
  } else {
    board.vram[1].read_bit(address - 0x1000)
  };
  let bit1 = if address < 0x1000 {
    board.vram[2].read_bit(address)
  } else {
    board.vram[3].read_bit(address - 0x1000)
  };
  (bit0, bit1)
}
