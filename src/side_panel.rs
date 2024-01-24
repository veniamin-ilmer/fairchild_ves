use chips::fairchild_f8;

pub struct SidePanel {
  registers: web_sys::HtmlCollection,
  memory: [u8; 0x40],
}

impl SidePanel {
  
  pub fn new() -> Self {
    Self {
      registers: get_tr_list("registers"),
      memory: [0; 0x40]
    }
  }
  
  pub fn print_memory(&mut self, board: &fairchild_f8::Board) {
    let tr_list = &self.registers;
    let regs = board.cpu.regs;
    for high in 0_usize..=0x7 {
      for low in 0_usize..=0x07 {
        let address = low + high * 0o10;
        if self.memory[address] != regs[address] {
          self.memory[address] = regs[address];
          let row_index = 1 + high as u32;
          let td_list = tr_list.item(row_index).expect("can't get tr").children();
          let col_index = low as u32 + 2;
          if let Some(td) = td_list.item(col_index) {
            td.set_text_content(Some(&format!("{:02X}", regs[address])));
          }
        }
      }
    }
  }
}

fn get_tr_list(table_id: &str) -> web_sys::HtmlCollection {
  let window = web_sys::window().expect("no global `window` exists");
  let document = window.document().expect("should have a document on window");
  let table = document.get_element_by_id(table_id).expect("can't find table id");
  let tbody = table.children().item(1).expect("can't get tbody");
  tbody.children()
}