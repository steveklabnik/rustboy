use mem;

//
// Joypad
//

static INPUT_RIGHT:           u8 = 0b0000_0001;
static INPUT_LEFT:            u8 = 0b0000_0010;
static INPUT_UP:              u8 = 0b0000_0100;
static INPUT_DOWN:            u8 = 0b0000_1000;
static INPUT_BUTTON_A:        u8 = 0b0000_0001;
static INPUT_BUTTON_B:        u8 = 0b0000_0010;
static INPUT_SELECT:          u8 = 0b0000_0100;
static INPUT_START:           u8 = 0b0000_1000;
static SELECT_DIRECTION_KEYS: u8 = 0b0001_0000;
static SELECT_BUTTON_KEYS:    u8 = 0b0010_0000;

static INPUT_MASK:            u8 = 0b0000_1111;
static SELECT_MASK:           u8 = 0b0011_0000;

pub enum Button {
  Right = 0,
  Left = 1,
  Up = 2,
  Down = 3,
  ButtonA = 4,
  ButtonB = 5,
  Select = 6,
  Start = 7,
}

pub struct Joypad {
  p1: u8,       // P1 register
  pressed: [bool, ..8],  // Button pressed state
}

impl Joypad {
  pub fn new() -> Joypad {
    Joypad { p1: 0xcf, pressed: [false, ..8] }
  }

  pub fn set_button(&mut self, button: Button, pressed: bool) {
    self.pressed[button as uint] = pressed;
    self.update_input();
  }

  pub fn reset(&mut self) {
    for p in self.pressed.iter_mut() {
      *p = false;
    }
  }

  fn update_input(&mut self) {
    // All bits are low-active, i.e. 0 means selected/pressed
    let mut input = INPUT_MASK;

    if (self.p1 & SELECT_DIRECTION_KEYS) == 0 {
      if self.pressed[Right as uint] {
        input &= !INPUT_RIGHT;
      }
      if self.pressed[Left as uint] {
        input &= !INPUT_LEFT;
      }
      if self.pressed[Up as uint] {
        input &= !INPUT_UP;
      }
      if self.pressed[Down as uint] {
        input &= !INPUT_DOWN;
      }
    }

    if (self.p1 & SELECT_BUTTON_KEYS) == 0 {
      if self.pressed[ButtonA as uint] {
        input &= !INPUT_BUTTON_A;
      }
      if self.pressed[ButtonB as uint] {
        input &= !INPUT_BUTTON_B;
      }
      if self.pressed[Select as uint] {
        input &= !INPUT_SELECT;
      }
      if self.pressed[Start as uint] {
        input &= !INPUT_START;
      }
    }

    self.p1 = (input & INPUT_MASK) | (self.p1 & !INPUT_MASK);
  }
}

impl mem::Mem for Joypad {
  fn loadb(&mut self, addr: u16) -> u8 {
    if addr != 0xff00 {
      fail!("invalid joypad register");
    }

    self.p1
  }

  fn storeb(&mut self, addr: u16, val: u8) {
    if addr != 0xff00 {
      fail!("invalid joypad register");
    }

    self.p1 = (val & SELECT_MASK) | (self.p1 & !SELECT_MASK);

    self.update_input();
  }
}
