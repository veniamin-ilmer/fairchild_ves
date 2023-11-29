use boards::fairchild_ves;
use wasm_bindgen::prelude::*;

#[derive(Default)]
struct PressedButtons {
  console1: Action,
  console2: Action,
  console3: Action,
  console4: Action,
  left: Action,
  right: Action,
  backward: Action,
  forward: Action,
  anticlock: Action,
  clock: Action,
  push: Action,
  pull: Action,
}

pub(super) struct Keyboard {
  pressed_buttons: PressedButtons,
  pending_button_var: wasm_bindgen::JsValue,
  pending_move_var: wasm_bindgen::JsValue,
  pending_wheel_var: wasm_bindgen::JsValue,
  cycle_count: u8,
}

#[derive(Default, PartialEq)]
enum Action {
  #[default]
  None,
  Keyboard,
  Mouse,
}

impl Keyboard {
  pub fn new() -> Self {
    let pending_button_var = js_sys::Reflect::get(
      &wasm_bindgen::JsValue::from(web_sys::window().unwrap()),
      &wasm_bindgen::JsValue::from("getPendingButton"),
    ).unwrap();
    
    let pending_move_var = js_sys::Reflect::get(
      &wasm_bindgen::JsValue::from(web_sys::window().unwrap()),
      &wasm_bindgen::JsValue::from("getPendingMove"),
    ).unwrap();
    
    let pending_wheel_var = js_sys::Reflect::get(
      &wasm_bindgen::JsValue::from(web_sys::window().unwrap()),
      &wasm_bindgen::JsValue::from("getPendingWheel"),
    ).unwrap();
    
    Self {
      pressed_buttons: Default::default(),
      pending_button_var: pending_button_var,
      pending_move_var: pending_move_var,
      pending_wheel_var: pending_wheel_var,
      cycle_count: 0,
    }
  }
  
  pub fn run_cycle(&self, board: &mut fairchild_ves::Board) {
    //buttons are inversed 4 low bits
    board.ports[0] = (board.ports[0] & 0b11110000) | console_to_port(&self.pressed_buttons); //Clear the console buttons before setting them below..
    
    if (board.cpu.ports[0] | board.ports[0]) & 0b01000000 == 0 {  //Not writing to video. Listening to controllers.
      board.ports[1] = controller_to_port(&self.pressed_buttons);  //Right controller
      board.ports[4] = 0b11111111;  //Left controller
    } else {  //Writing to video. Clear controller
      board.ports[1] = 0;  //Right controller
      board.ports[4] = 0;  //Left controller
    }
  }
  

  
  pub fn run_refresh_cycle(&mut self, board: &mut fairchild_ves::Board) {
    let (x, y) = self.get_movement();
    //Mouse shouldn't interrupt keyboard press
    if self.pressed_buttons.left != Action::Keyboard
    && self.pressed_buttons.right != Action::Keyboard
    && self.pressed_buttons.forward != Action::Keyboard
    && self.pressed_buttons.backward != Action::Keyboard {
      if x < 0.0 {
        self.pressed_buttons.left = Action::Mouse;
        self.pressed_buttons.right = Action::None;
      } else if x > 0.0 {
        self.pressed_buttons.left = Action::None;
        self.pressed_buttons.right = Action::Mouse;
      } else {
        self.pressed_buttons.left = Action::None;
        self.pressed_buttons.right = Action::None;
      }
      if y < 0.0 {
        self.pressed_buttons.forward = Action::Mouse;
        self.pressed_buttons.backward = Action::None;
      } else if y > 0.0 {
        self.pressed_buttons.forward = Action::None;
        self.pressed_buttons.backward = Action::Mouse;
      } else {
        self.pressed_buttons.forward = Action::None;
        self.pressed_buttons.backward = Action::None;
      }
    }
    //Mouse shouldn't interrupt keyboard press
    if self.pressed_buttons.anticlock != Action::Keyboard
    && self.pressed_buttons.clock != Action::Keyboard {
      self.cycle_count += 1;
      if self.cycle_count == 3 {
        let wheel = self.get_wheel();
        if wheel > 0.0 {
          self.pressed_buttons.anticlock = Action::Mouse;
          self.pressed_buttons.clock = Action::None;
        } else if wheel < 0.0 {
          self.pressed_buttons.anticlock = Action::None;
          self.pressed_buttons.clock = Action::Mouse;
        } else {
          self.pressed_buttons.anticlock = Action::None;
          self.pressed_buttons.clock = Action::None;
        }
        self.cycle_count = 0;
      }
    }
  
    //Keyboard arrow keys overrides mouse
    if let Some((scan_code, press_type)) = self.get_keypress() {
      if scan_code == 255 && press_type != Action::None {
        board.cpu.reset = true;
        self.pressed_buttons = Default::default();  //Since we are reseting, let's reset all buttons just in case...
      } else {
        match scan_code {
          1 => self.pressed_buttons.console1 = press_type,
          2 => self.pressed_buttons.console2 = press_type,
          3 => self.pressed_buttons.console3 = press_type,
          4 => self.pressed_buttons.console4 = press_type,
          5 => self.pressed_buttons.push = press_type, //Left mouse
          6 => self.pressed_buttons.pull = press_type, //Right mouse
          7 => self.pressed_buttons.left = press_type,
          8 => self.pressed_buttons.right = press_type,
          9 => self.pressed_buttons.forward = press_type,
          10 => self.pressed_buttons.backward = press_type,
          11 => self.pressed_buttons.clock = press_type,
          12 => self.pressed_buttons.anticlock = press_type,
          255 => (),  //Restart
          _ => unimplemented!(),
        }
      }
    }
  }
  
   //Wish there were a way to get an integer directly without needing to go through a float...
  fn get_keypress(&self) -> Option<(u8, Action)> {
    let pending_func: &js_sys::Function = self.pending_button_var.dyn_ref().unwrap();
    let click_var = pending_func.apply(&JsValue::null(), &js_sys::Array::new()).unwrap();
    if let Some(click_array) = click_var.dyn_ref::<js_sys::Array>() {
      if click_array.length() == 2 {
        if let Some(code) = click_array.get(0).as_f64() {
          if let Some(press_type) = click_array.get(1).as_f64() {
            if press_type.round() as u8 == 1 {
              Some((code.round() as u8, Action::Keyboard))
            } else {
              Some((code.round() as u8, Action::None))
            }
          } else {
            None
          }
        } else {
          None
        }
      } else {
        None
      }
    } else {
      None
    }
  }
  
  //Only grabs the latest mouse position
  fn get_movement(&self) -> (f64, f64) {
    let pending_func: &js_sys::Function = self.pending_move_var.dyn_ref().unwrap();
    let movements_var = pending_func.apply(&JsValue::null(), &js_sys::Array::new()).unwrap();
    if let Some(movements_array) = movements_var.dyn_ref::<js_sys::Array>() {
      if movements_array.length() == 2 {  //There is data
        if let Some(x) = movements_array.get(0).as_f64() {
          if let Some(y) = movements_array.get(1).as_f64() {
            return (x, y);
          }
        }
      }
    }
    (0.0, 0.0)
  }
  
  fn get_wheel(&self) -> f64 {
    let pending_func: &js_sys::Function = self.pending_wheel_var.dyn_ref().unwrap();
    let wheel_var = pending_func.apply(&JsValue::null(), &js_sys::Array::new()).unwrap();
    wheel_var.as_f64().unwrap_or(0.0)
  }
}

fn console_to_port(pressed_buttons: &PressedButtons) -> u8 {
  let mut value = 0b00001111;
  if pressed_buttons.console1 != Action::None {
    value &= 0b00001110;
  }
  if pressed_buttons.console2 != Action::None {
    value &= 0b00001101;
  }
  if pressed_buttons.console3 != Action::None {
    value &= 0b00001011;
  }
  if pressed_buttons.console4 != Action::None {
    value &= 0b00000111;
  }
  value
}

fn controller_to_port(pressed_buttons: &PressedButtons) -> u8 {
  let mut value = 0b11111111;
  if pressed_buttons.right != Action::None {
    value &= 0b11111110;
  }
  if pressed_buttons.left != Action::None {
    value &= 0b11111101;
  }
  if pressed_buttons.backward != Action::None {
    value &= 0b11111011;
  }
  if pressed_buttons.forward != Action::None {
    value &= 0b11110111;
  }
  if pressed_buttons.anticlock != Action::None {
    value &= 0b11101111;
  }
  if pressed_buttons.clock != Action::None {
    value &= 0b11011111;
  }
  if pressed_buttons.pull != Action::None {
    value &= 0b10111111;
  }
  if pressed_buttons.push != Action::None {
    value &= 0b01111111;
  }
  value
}
