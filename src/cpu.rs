use mem;

//
// Statics
//

const CARRY_OFFSET:      uint = 4;
const HALF_CARRY_OFFSET: uint = 5;
const ADD_SUB_OFFSET:    uint = 6;
const ZERO_OFFSET:       uint = 7;

const CARRY_FLAG:        u8 = 1 << CARRY_OFFSET;
const HALF_CARRY_FLAG:   u8 = 1 << HALF_CARRY_OFFSET;
const ADD_SUB_FLAG:      u8 = 1 << ADD_SUB_OFFSET;
const ZERO_FLAG:         u8 = 1 << ZERO_OFFSET;

pub const CYCLES_PER_SEC: uint = 4194304; // 4.194304 MHz


//
// Registers
//

pub struct Regs {
  pub a: u8,
  pub b: u8,
  pub c: u8,
  pub d: u8,
  pub e: u8,
  pub h: u8,
  pub l: u8,
  pub f: u8,
  pub sp: u16,
  pub pc: u16,
}

impl Regs {
  fn new() -> Regs {
    Regs { a: 0, b: 0, c: 0, d: 0, e: 0, h: 0, l: 0, f: 0, sp: 0, pc: 0 }
  }
}


//
// Instruction decoding
//

#[deriving(Show)]
pub enum Reg8 {
  A, B, C, D, E, H, L
}

impl<M: mem::Mem> Reg8 {
  fn load(&self, cpu: &Cpu<M>) -> u8 {
    match *self {
      A => cpu.regs.a,
      B => cpu.regs.b,
      C => cpu.regs.c,
      D => cpu.regs.d,
      E => cpu.regs.e,
      H => cpu.regs.h,
      L => cpu.regs.l,
    }
  }

  fn store(&self, cpu: &mut Cpu<M>, val: u8) {
    match *self {
      A => cpu.regs.a = val,
      B => cpu.regs.b = val,
      C => cpu.regs.c = val,
      D => cpu.regs.d = val,
      E => cpu.regs.e = val,
      H => cpu.regs.h = val,
      L => cpu.regs.l = val,
    }
  }
}

#[deriving(Show)]
pub enum Reg16 {
  AF, BC, DE, HL, SP, PC
}

impl<M: mem::Mem> Reg16 {
  fn load(&self, cpu: &Cpu<M>) -> u16 {
    match *self {
      AF => (cpu.regs.a as u16 << 8) | cpu.regs.f as u16,
      BC => (cpu.regs.b as u16 << 8) | cpu.regs.c as u16,
      DE => (cpu.regs.d as u16 << 8) | cpu.regs.e as u16,
      HL => (cpu.regs.h as u16 << 8) | cpu.regs.l as u16,
      SP => cpu.regs.sp,
      PC => cpu.regs.pc,
    }
  }

  fn store(&self, cpu: &mut Cpu<M>, val: u16) {
    match *self {
      AF => { cpu.regs.a = (val >> 8) as u8; cpu.regs.f = val as u8 & 0xf0 }
      BC => { cpu.regs.b = (val >> 8) as u8; cpu.regs.c = val as u8 }
      DE => { cpu.regs.d = (val >> 8) as u8; cpu.regs.e = val as u8 }
      HL => { cpu.regs.h = (val >> 8) as u8; cpu.regs.l = val as u8 }
      SP => cpu.regs.sp = val,
      PC => cpu.regs.pc = val,
    }
  }
}

pub enum Addr8 {
  Imm8(u8),
  Ind8(u8),
  Imm16Ind8(u16),
  Reg8Dir(Reg8),
  Reg8Ind8(Reg8),
  Reg16Ind8(Reg16),
  Reg16Ind8Inc(Reg16),
  Reg16Ind8Dec(Reg16),
}

impl Addr8 {
  fn cycles(&self) -> u8 {
    // Every (byte) memory access costs 4 cycles
    match *self {
      Reg8Dir(_)        => 0, // register access is "free"
      Imm8(_) |
      Ind8(_) |
      Reg8Ind8(_)       => 4,  // one memory access
      Reg16Ind8(_) |
      Reg16Ind8Inc(_) |
      Reg16Ind8Dec(_)   => 8,  // two memory accesses
      Imm16Ind8(_)      => 12, // three memory accesses
    }
  }

  fn load<M: mem::Mem>(&self, cpu: &mut Cpu<M>) -> u8 {
    match *self {
      Imm8(val)       => val,
      Ind8(offset)    => cpu.mem.loadb(0xff00 + offset as u16),
      Imm16Ind8(addr) => cpu.mem.loadb(addr),
      Reg8Dir(r)      => r.load(cpu),
      Reg8Ind8(r)     => {
        let offset = r.load(cpu);
        cpu.mem.loadb(0xff00 + offset as u16)
      }
      Reg16Ind8(r) |
      Reg16Ind8Inc(r) |
      Reg16Ind8Dec(r) => {
        let addr = r.load(cpu);
        let result = cpu.mem.loadb(addr);
        match *self {
          Reg16Ind8Inc(_) => r.store(cpu, addr + 1),
          Reg16Ind8Dec(_) => r.store(cpu, addr - 1),
          _ => ()
        }
        result
      }
    }
  }

  fn store<M: mem::Mem>(&self, cpu: &mut Cpu<M>, val: u8) {
    match *self {
      Ind8(offset)    => cpu.mem.storeb(0xff00 + offset as u16, val),
      Imm16Ind8(addr) => cpu.mem.storeb(addr, val),
      Reg8Dir(r)      => r.store(cpu, val),
      Reg8Ind8(r)     => {
        let offset = r.load(cpu);
        cpu.mem.storeb(0xff00 + offset as u16, val)
      }
      Reg16Ind8(r) |
      Reg16Ind8Inc(r) |
      Reg16Ind8Dec(r) => {
        let addr = r.load(cpu);
        cpu.mem.storeb(addr, val);
        match *self {
          Reg16Ind8Inc(_) => r.store(cpu, addr + 1),
          Reg16Ind8Dec(_) => r.store(cpu, addr - 1),
          _ => ()
        }
      }
      _  => panic!("invalid addressing mode for 8-bit store")
    }
  }
}

pub enum Addr16 {
  Imm16(u16),
  Ind16(u16),
  Reg16Dir(Reg16),
}

impl Addr16 {
  fn cycles(&self) -> u8 {
    // Every (byte) memory access costs 4 cycles
    match *self {
      Reg16Dir(_)   => 0,  // register access is "free"
      Imm16(_)      => 8,  // two memory accesses
      Ind16(_)      => 16, // four memory accesses
    }
  }

  fn load<M: mem::Mem>(&self, cpu: &mut Cpu<M>) -> u16 {
    match *self {
      Imm16(val)    => val,
      Ind16(addr)   => cpu.mem.loadw(addr),
      Reg16Dir(r)   => r.load(cpu),
    }
  }

  fn store<M: mem::Mem>(&self, cpu: &mut Cpu<M>, val: u16) {
    match *self {
      Ind16(addr)   => cpu.mem.storew(addr, val),
      Reg16Dir(r)   => r.store(cpu, val),
      _ => panic!("invalid addressing mode for 16-bit store")
    }
  }
}

pub enum Cond {
  CondNone,
  CondZ,
  CondNZ,
  CondC,
  CondNC
}

impl Cond {
  fn eval<M: mem::Mem>(self, cpu: &Cpu<M>) -> bool {
    match self {
      CondNone => true,
      CondZ    => (cpu.regs.f & ZERO_FLAG) != 0,
      CondNZ   => (cpu.regs.f & ZERO_FLAG) == 0,
      CondC    => (cpu.regs.f & CARRY_FLAG) != 0,
      CondNC   => (cpu.regs.f & CARRY_FLAG) == 0,
    }
  }
}

pub trait Decoder<R> {
  fn fetch(&mut self) -> u8;

  // Misc/control
  fn nop (&mut self)          -> R;

  fn ei  (&mut self)          -> R;
  fn di  (&mut self)          -> R;

  fn halt(&mut self)          -> R;
  fn stop(&mut self, val: u8) -> R;

  // Jump/call
  fn jp  (&mut self, Cond, addr: Addr16) -> R;
  fn jr  (&mut self, Cond, rel: i8)      -> R;

  fn call(&mut self, Cond, addr: Addr16) -> R;
  fn rst (&mut self, addr: u8)           -> R;

  fn ret (&mut self, Cond)               -> R;
  fn reti(&mut self)                     -> R;

  // Load/store/move
  fn ld8 (&mut self, dst: Addr8,  src: Addr8)  -> R;
  fn ld16(&mut self, dst: Addr16, src: Addr16) -> R;
  fn ldh (&mut self, dst: Addr8,  src: Addr8)  -> R;
  fn ldhl(&mut self, rel: i8)                  -> R;

  fn push(&mut self, src: Addr16)              -> R;
  fn pop (&mut self, dst: Addr16)              -> R;

  // Arithmetic/logic
  fn add8 (&mut self, src: Addr8)               -> R;
  fn add16(&mut self, dst: Addr16, src: Addr16) -> R;
  fn addsp(&mut self, rel: i8)                  -> R;
  fn adc  (&mut self, src: Addr8)               -> R;

  fn sub  (&mut self, src: Addr8)               -> R;
  fn sbc  (&mut self, src: Addr8)               -> R;

  fn inc8 (&mut self, dst: Addr8)               -> R;
  fn inc16(&mut self, dst: Addr16)              -> R;
  fn dec8 (&mut self, dst: Addr8)               -> R;
  fn dec16(&mut self, dst: Addr16)              -> R;

  fn and  (&mut self, src: Addr8)               -> R;
  fn or   (&mut self, src: Addr8)               -> R;
  fn xor  (&mut self, src: Addr8)               -> R;

  fn cp   (&mut self, src: Addr8)               -> R;

  fn cpl  (&mut self)                           -> R;

  fn scf  (&mut self)                           -> R;
  fn ccf  (&mut self)                           -> R;

  fn daa  (&mut self)                           -> R;

  // Rotation/shift/bit
  fn rlca(&mut self)                      -> R;
  fn rla (&mut self)                      -> R;
  fn rrca(&mut self)                      -> R;
  fn rra (&mut self)                      -> R;

  fn rlc (&mut self, dst: Addr8)          -> R;
  fn rl  (&mut self, dst: Addr8)          -> R;
  fn rrc (&mut self, dst: Addr8)          -> R;
  fn rr  (&mut self, dst: Addr8)          -> R;

  fn sla (&mut self, dst: Addr8)          -> R;
  fn sra (&mut self, dst: Addr8)          -> R;
  fn srl (&mut self, dst: Addr8)          -> R;

  fn bit (&mut self, bit: u8, src: Addr8) -> R;
  fn res (&mut self, bit: u8, dst: Addr8) -> R;
  fn set (&mut self, bit: u8, dst: Addr8) -> R;

  fn swap(&mut self, dst: Addr8)          -> R;

  // Undefined/illegal
  fn undef(&mut self, opcode: u8) -> R;
}

// Several instructions use a 3 bit part of the opcode to encode common
// operands, this method decodes the operand from the lower 3 bits
fn decode_addr(code: u8) -> Addr8 {
  match code & 0x07 {
    0x00 => Reg8Dir(B),
    0x01 => Reg8Dir(C),
    0x02 => Reg8Dir(D),
    0x03 => Reg8Dir(E),
    0x04 => Reg8Dir(H),
    0x05 => Reg8Dir(L),
    0x06 => Reg16Ind8(HL),
    0x07 => Reg8Dir(A),
    _    => panic!("logic error"),
  }
}

// Source: http://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
pub fn decode<R, D: Decoder<R>>(d: &mut D) -> R {
  let fetchw = |d: &mut D| -> u16 {
    let lo = d.fetch();
    let hi = d.fetch();
    (hi as u16 << 8) | lo as u16
  };

  let opcode = d.fetch();
  match opcode {
    // 0x00
    0x00 => d.nop(),
    0x01 => { let imm = fetchw(d); d.ld16(Reg16Dir(BC), Imm16(imm)) }
    0x02 => d.ld8(Reg16Ind8(BC), Reg8Dir(A)),
    0x03 => d.inc16(Reg16Dir(BC)),
    0x04 => d.inc8(Reg8Dir(B)),
    0x05 => d.dec8(Reg8Dir(B)),
    0x06 => { let imm = d.fetch(); d.ld8(Reg8Dir(B), Imm8(imm)) }
    0x07 => d.rlca(),

    0x08 => { let ind = fetchw(d); d.ld16(Ind16(ind), Reg16Dir(SP)) }
    0x09 => d.add16(Reg16Dir(HL), Reg16Dir(BC)),
    0x0a => d.ld8(Reg8Dir(A), Reg16Ind8(BC)),
    0x0b => d.dec16(Reg16Dir(BC)),
    0x0c => d.inc8(Reg8Dir(C)),
    0x0d => d.dec8(Reg8Dir(C)),
    0x0e => { let imm = d.fetch(); d.ld8(Reg8Dir(C), Imm8(imm)) }
    0x0f => d.rrca(),

    // 0x10
    0x10 => { let val = d.fetch(); d.stop(val) }
    0x11 => { let imm = fetchw(d); d.ld16(Reg16Dir(DE), Imm16(imm)) }
    0x12 => d.ld8(Reg16Ind8(DE), Reg8Dir(A)),
    0x13 => d.inc16(Reg16Dir(DE)),
    0x14 => d.inc8(Reg8Dir(D)),
    0x15 => d.dec8(Reg8Dir(D)),
    0x16 => { let imm = d.fetch(); d.ld8(Reg8Dir(D), Imm8(imm)) }
    0x17 => d.rla(),

    0x18 => { let imm = d.fetch(); d.jr(CondNone, imm as i8) }
    0x19 => d.add16(Reg16Dir(HL), Reg16Dir(DE)),
    0x1a => d.ld8(Reg8Dir(A), Reg16Ind8(DE)),
    0x1b => d.dec16(Reg16Dir(DE)),
    0x1c => d.inc8(Reg8Dir(E)),
    0x1d => d.dec8(Reg8Dir(E)),
    0x1e => { let imm = d.fetch(); d.ld8(Reg8Dir(E), Imm8(imm)) }
    0x1f => d.rra(),

    // 0x20
    0x20 => { let imm = d.fetch(); d.jr(CondNZ, imm as i8) }
    0x21 => { let imm = fetchw(d); d.ld16(Reg16Dir(HL), Imm16(imm)) }
    0x22 => d.ld8(Reg16Ind8Inc(HL), Reg8Dir(A)),
    0x23 => d.inc16(Reg16Dir(HL)),
    0x24 => d.inc8(Reg8Dir(H)),
    0x25 => d.dec8(Reg8Dir(H)),
    0x26 => { let imm = d.fetch(); d.ld8(Reg8Dir(H), Imm8(imm)) }
    0x27 => d.daa(),

    0x28 => { let imm = d.fetch(); d.jr(CondZ, imm as i8) }
    0x29 => d.add16(Reg16Dir(HL), Reg16Dir(HL)),
    0x2a => d.ld8(Reg8Dir(A), Reg16Ind8Inc(HL)),
    0x2b => d.dec16(Reg16Dir(HL)),
    0x2c => d.inc8(Reg8Dir(L)),
    0x2d => d.dec8(Reg8Dir(L)),
    0x2e => { let imm = d.fetch(); d.ld8(Reg8Dir(L), Imm8(imm)) }
    0x2f => d.cpl(),

    // 0x30
    0x30 => { let imm = d.fetch(); d.jr(CondNC, imm as i8) }
    0x31 => { let imm = fetchw(d); d.ld16(Reg16Dir(SP), Imm16(imm)) }
    0x32 => d.ld8(Reg16Ind8Dec(HL), Reg8Dir(A)),
    0x33 => d.inc16(Reg16Dir(SP)),
    0x34 => d.inc8(Reg16Ind8(HL)),
    0x35 => d.dec8(Reg16Ind8(HL)),
    0x36 => { let imm = d.fetch(); d.ld8(Reg16Ind8(HL), Imm8(imm)) }
    0x37 => d.scf(),

    0x38 => { let imm = d.fetch(); d.jr(CondC, imm as i8) }
    0x39 => d.add16(Reg16Dir(HL), Reg16Dir(SP)),
    0x3a => d.ld8(Reg8Dir(A), Reg16Ind8Dec(HL)),
    0x3b => d.dec16(Reg16Dir(SP)),
    0x3c => d.inc8(Reg8Dir(A)),
    0x3d => d.dec8(Reg8Dir(A)),
    0x3e => { let imm = d.fetch(); d.ld8(Reg8Dir(A), Imm8(imm)) }
    0x3f => d.ccf(),

    // 0x40-0x70
    0x40...0x75 | 0x77...0x7f
         => d.ld8(decode_addr(opcode >> 3), decode_addr(opcode)),
    0x76 => d.halt(),

    // 0x80
    0x80...0x87 => d.add8(decode_addr(opcode)),
    0x88...0x8f => d.adc(decode_addr(opcode)),

    // 0x90
    0x90...0x97 => d.sub(decode_addr(opcode)),
    0x98...0x9f => d.sbc(decode_addr(opcode)),

    // 0xa0
    0xa0...0xa7 => d.and(decode_addr(opcode)),
    0xa8...0xaf => d.xor(decode_addr(opcode)),

    // 0xb0
    0xb0...0xb7 => d.or(decode_addr(opcode)),
    0xb8...0xbf => d.cp(decode_addr(opcode)),

    // 0xc0
    0xc0 => d.ret(CondNZ),
    0xc1 => d.pop(Reg16Dir(BC)),
    0xc2 => { let imm = fetchw(d); d.jp(CondNZ, Imm16(imm)) }
    0xc3 => { let imm = fetchw(d); d.jp(CondNone, Imm16(imm)) }
    0xc4 => { let imm = fetchw(d); d.call(CondNZ, Imm16(imm)) }
    0xc5 => d.push(Reg16Dir(BC)),
    0xc6 => { let imm = d.fetch(); d.add8(Imm8(imm)) }
    0xc7 => d.rst(0x00),

    0xc8 => d.ret(CondZ),
    0xc9 => d.ret(CondNone),
    0xca => { let imm = fetchw(d); d.jp(CondZ, Imm16(imm)) }
    0xcb => {
      let extra = d.fetch();
      let addr = decode_addr(extra);

      match extra & 0xf8 {
        0x00 => d.rlc(addr),
        0x08 => d.rrc(addr),
        0x10 => d.rl(addr),
        0x18 => d.rr(addr),
        0x20 => d.sla(addr),
        0x28 => d.sra(addr),
        0x30 => d.swap(addr),
        0x38 => d.srl(addr),
        0x40...0x78 =>
                d.bit((extra >> 3) & 0b111, addr),
        0x80...0xb8 =>
                d.res((extra >> 3) & 0b111, addr),
        0xc0...0xf8 =>
                d.set((extra >> 3) & 0b111, addr),
        _ => panic!("logic error")
      }
    }
    0xcc => { let imm = fetchw(d); d.call(CondZ, Imm16(imm)) }
    0xcd => { let imm = fetchw(d); d.call(CondNone, Imm16(imm)) }
    0xce => { let imm = d.fetch(); d.adc(Imm8(imm)) }
    0xcf => d.rst(0x08),

    // 0xd0
    0xd0 => d.ret(CondNC),
    0xd1 => d.pop(Reg16Dir(DE)),
    0xd2 => { let imm = fetchw(d); d.jp(CondNC, Imm16(imm)) }
    /*0xd3 => d.nop(),*/
    0xd4 => { let imm = fetchw(d); d.call(CondNC, Imm16(imm)) }
    0xd5 => d.push(Reg16Dir(DE)),
    0xd6 => { let imm = d.fetch(); d.sub(Imm8(imm)) }
    0xd7 => d.rst(0x10),

    0xd8 => d.ret(CondC),
    0xd9 => d.reti(),
    0xda => { let imm = fetchw(d); d.jp(CondC, Imm16(imm)) }
    /*0xdb => d.nop(),*/
    0xdc => { let imm = fetchw(d); d.call(CondC, Imm16(imm)) }
    /*0xdd => d.nop(),*/
    0xde => { let imm = d.fetch(); d.sbc(Imm8(imm)) }
    0xdf => d.rst(0x18),

    // 0xe0
    0xe0 => { let ind = d.fetch(); d.ldh(Ind8(ind), Reg8Dir(A)) }
    0xe1 => d.pop(Reg16Dir(HL)),
    0xe2 => d.ld8(Reg8Ind8(C), Reg8Dir(A)),
    /*0xe3 => d.nop(),*/
    /*0xe4 => d.nop(),*/
    0xe5 => d.push(Reg16Dir(HL)),
    0xe6 => { let imm = d.fetch(); d.and(Imm8(imm)) }
    0xe7 => d.rst(0x20),

    0xe8 => { let imm = d.fetch(); d.addsp(imm as i8) }
    0xe9 => d.jp(CondNone, Reg16Dir(HL)),
    0xea => { let ind = fetchw(d); d.ld8(Imm16Ind8(ind), Reg8Dir(A)) }
    /*0xeb => d.nop(),*/
    /*0xec => d.nop(),*/
    /*0xed => d.nop(),*/
    0xee => { let imm = d.fetch(); d.xor(Imm8(imm)) }
    0xef => d.rst(0x28),

    // 0xf0
    0xf0 => { let ind = d.fetch(); d.ldh(Reg8Dir(A), Ind8(ind)) }
    0xf1 => d.pop(Reg16Dir(AF)),
    0xf2 => d.ld8(Reg8Dir(A), Reg8Ind8(C)),
    0xf3 => d.di(),
    /*0xf4 => d.nop(),*/
    0xf5 => d.push(Reg16Dir(AF)),
    0xf6 => { let imm = d.fetch(); d.or(Imm8(imm)) }
    0xf7 => d.rst(0x30),

    0xf8 => { let imm = d.fetch(); d.ldhl(imm as i8) }
    0xf9 => d.ld16(Reg16Dir(SP), Reg16Dir(HL)),
    0xfa => { let ind = fetchw(d); d.ld8(Reg8Dir(A), Imm16Ind8(ind)) }
    0xfb => d.ei(),
    /*0xfc => d.nop(),*/
    /*0xfd => d.nop(),*/
    0xfe => { let imm = d.fetch(); d.cp(Imm8(imm)) }
    0xff => d.rst(0x38),

    _    => d.undef(opcode)
  }
}

//
// CPU
//

pub struct Cpu<M> {
  pub regs: Regs,
  ime: bool,
  halted: bool,
  pub cycles: u64,
  pub mem: M,
}

impl<M: mem::Mem> Cpu<M> {
  pub fn new(mem: M) -> Cpu<M> {
    Cpu {
      regs: Regs::new(),
      ime: false, // TODO: Are interrupts enabled at boot?
      halted: false,
      cycles: 0u64,
      mem: mem,
    }
  }

  pub fn step(&mut self) -> u8 {
    if self.halted {
      if self.mem.loadb(0xff0f) != 0 {
        // Wake up on interrupt
        self.halted = false;
      } else {
        // NOP
        self.cycles += 4;
        return 4;
      }
    }

    // Check for interrupts
    if self.ime {
      let enabled_mask = self.mem.loadb(0xffff);
      let request_mask = self.mem.loadb(0xff0f);
      let effective = enabled_mask & request_mask;
       // Get the lowest set bit, which is the IRQ with the highest priority
      let irq = effective.trailing_zeros();
      if irq < 5 {
        // Valid IRQ (0-4)
        self.push(Reg16Dir(PC));                    // Push PC
        self.regs.pc = 0x40 + irq as u16 * 0x08; // Jump to ISR
        self.ime = false;                        // Disable interrupts
        self.mem.storeb(0xff0f, request_mask & !(1 << irq as uint)); // Clear IRQ flag
        self.cycles += 20;
        return 20 // 5 machine cycles, according to GBCPUman.pdf
      }
    }

    let elapsed = decode(self);
    self.cycles += elapsed as u64;
    elapsed
  }

  // Flags helpers

  fn get_flag(&self, flag: u8) -> bool {
    (self.regs.f & flag) != 0
  }

  fn set_flag(&mut self, flag: u8, on: bool) {
    if on {
      self.regs.f |= flag;
    } else {
      self.regs.f &= !flag;
    }
  }

  // Addition helper
  fn add_(&mut self, a: u8, b: u8, c: u8) {
    let result = a as u32 + b as u32 + c as u32;
    self.regs.a = result as u8;
    let zero = self.regs.a == 0;
    self.set_flag(ZERO_FLAG, zero);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, (((a & 0xf) + (b & 0xf) + c) & 0x10) != 0);
    self.set_flag(CARRY_FLAG, (result & 0x100) != 0);
  }

  // Subtraction helper
  fn sub_(&mut self, a: u8, b: u8, c: u8) {
    let result = a as u32 - b as u32 - c as u32;
    self.regs.a = result as u8;
    let zero = self.regs.a == 0;
    self.set_flag(ZERO_FLAG, zero);
    self.set_flag(ADD_SUB_FLAG, true);
    self.set_flag(HALF_CARRY_FLAG, (((a & 0xf) - (b & 0xf) - c) & 0x10) != 0); // ???
    self.set_flag(CARRY_FLAG, (result & 0x100) != 0); // ???
  }

  // Add to SP helper
  fn addsp_(&mut self, rel: i8) -> u16 {
    let sp = self.regs.sp;
    let rel = rel as u16;
    let result = sp + rel;
    self.set_flag(ZERO_FLAG, false);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, (((sp & 0xf) + (rel & 0xf)) & 0x10) != 0);
    self.set_flag(CARRY_FLAG, (((sp & 0xff) + (rel & 0xff)) & 0x100) != 0);
    result
  }
}

// Opcode implementation.
//
// Returns number of elapsed cyles.
//
// Sources:
//   * http://www.z80.info/z80code.txt
//   * http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
//   * http://www.zilog.com/docs/z80/um0080.pdf
impl<M: mem::Mem> Decoder<u8> for Cpu<M> {
  fn fetch(&mut self) -> u8 {
    let result = self.mem.loadb(self.regs.pc);
    self.regs.pc += 1;
    result
  }

  //
  // Misc/control
  //

  fn nop(&mut self) -> u8 {
    4
  }

  fn ei(&mut self) -> u8{
    self.ime = true;
    4
  }

  fn di(&mut self) -> u8 {
    self.ime = false;
    4
  }

  fn halt(&mut self) -> u8 {
    self.halted = true;
    4
  }

  fn stop(&mut self, val: u8) -> u8 {
    //panic!("instruction not implemented: stop")
    4
  }

  //
  // Jump/call
  //

  fn jp(&mut self, cond: Cond, addr: Addr16) -> u8 {
    if cond.eval(self) {
      self.ld16(Reg16Dir(PC), addr);
    }
    4 + addr.cycles()
  }

  fn jr(&mut self, cond: Cond, rel: i8) -> u8 {
    if cond.eval(self) {
      let addr = self.regs.pc as i16 + rel as i16;
      self.ld16(Reg16Dir(PC), Imm16(addr as u16));
    }
    8
  }

  fn call(&mut self, cond: Cond, addr: Addr16) -> u8 {
    if cond.eval(self) {
      self.push(Reg16Dir(PC));
      self.ld16(Reg16Dir(PC), addr);
    }
    12
  }

  fn rst(&mut self, addr: u8) -> u8 {
    self.call(CondNone, Imm16(addr as u16));
    32
  }

  fn ret(&mut self, cond: Cond) -> u8 {
    if cond.eval(self) {
      self.pop(Reg16Dir(PC));
    }
    8
  }

  fn reti(&mut self) -> u8 {
    self.pop(Reg16Dir(PC));
    self.ei();
    8
  }

  //
  // Load/store/move
  //

  fn ld8(&mut self, dst: Addr8,  src: Addr8) -> u8 {
    let val = src.load(self);
    dst.store(self, val);
    4 + dst.cycles() + src.cycles()
  }

  fn ld16(&mut self, dst: Addr16, src: Addr16) -> u8 {
    let val = src.load(self);
    dst.store(self, val);
    4 + dst.cycles() + src.cycles()
    // TODO: LD SP, HL takes 8 cycles
  }

  fn ldh(&mut self, dst: Addr8, src: Addr8) -> u8 {
    self.ld8(dst, src);
    4 + dst.cycles() + src.cycles()
  }

  fn ldhl(&mut self, rel: i8) -> u8 {
    let result = self.addsp_(rel);
    self.regs.h = (result >> 8) as u8;
    self.regs.l = (result & 0xff) as u8;
    12
  }

  fn push(&mut self, src: Addr16) -> u8 {
    self.regs.sp -= 2;
    let val = src.load(self);
    self.mem.storew(self.regs.sp, val);
    16 + src.cycles()
  }

  fn pop(&mut self, dst: Addr16) -> u8 {
    let val = self.mem.loadw(self.regs.sp);
    dst.store(self, val);
    self.regs.sp += 2;
    12 + dst.cycles()
  }

  //
  // Arithmetic/logic
  //

  fn add8(&mut self, src: Addr8) -> u8 {
    let a = self.regs.a;
    let b = src.load(self);
    self.add_(a, b, 0);

    4 + src.cycles()
  }

  fn add16(&mut self, dst: Addr16, src: Addr16) -> u8 {
    let op1 = dst.load(self) as u32;
    let op2 = src.load(self) as u32;
    let result = op1 + op2;
    dst.store(self, result as u16);

    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, (((op1 & 0xfff) + (op2 & 0xfff)) & 0x1000) != 0);
    self.set_flag(CARRY_FLAG, (result & 0x10000) != 0);

    8
  }

  fn addsp(&mut self, rel: i8) -> u8 {
    self.regs.sp = self.addsp_(rel);

    16
  }

  fn adc(&mut self, src: Addr8) -> u8 {
    let a = self.regs.a;
    let b = src.load(self);
    let c = (self.regs.f >> CARRY_OFFSET) & 1;
    self.add_(a, b, c);

    4 + src.cycles()
  }

  fn sub(&mut self, src: Addr8) -> u8 {
    let a = self.regs.a;
    let b = src.load(self);
    self.sub_(a, b, 0);

    4 + src.cycles()
  }

  fn sbc(&mut self, src: Addr8) -> u8 {
    let a = self.regs.a;
    let b = src.load(self);
    let c = (self.regs.f >> CARRY_OFFSET) & 1;
    self.sub_(a, b, c);

    4 + src.cycles()
  }

  fn inc8(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    let result = val + 1;
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, val & 0xf == 0xf);
    // Note: carry flag not affected

    4 + 2 * dst.cycles()
  }

  fn inc16(&mut self, dst: Addr16) -> u8 {
    let val = dst.load(self);
    dst.store(self, val + 1);

    8
  }

  fn dec8(&mut self, dst: Addr8) -> u8 {
    let result = dst.load(self) - 1;
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, true);
    self.set_flag(HALF_CARRY_FLAG, result & 0xf == 0xf);
    // Note: carry flag is not affected

    4 + 2 * dst.cycles()
  }

  fn dec16(&mut self, dst: Addr16) -> u8 {
    let val = dst.load(self);
    dst.store(self, val - 1);

    8
  }

  fn and(&mut self, src: Addr8) -> u8 {
    self.regs.a &= src.load(self);

    let zero = self.regs.a == 0;
    self.set_flag(ZERO_FLAG, zero);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, true); // Yes, this is correct
    self.set_flag(CARRY_FLAG, false);

    4 + src.cycles()
  }

  fn or(&mut self, src: Addr8) -> u8 {
    self.regs.a |= src.load(self);

    let zero = self.regs.a == 0;
    self.set_flag(ZERO_FLAG, zero);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, false);

    4 + src.cycles()
  }

  fn xor(&mut self, src: Addr8) -> u8 {
    self.regs.a ^= src.load(self);

    let zero = self.regs.a == 0;
    self.set_flag(ZERO_FLAG, zero);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, false);

    4 + src.cycles()
  }

  fn cp(&mut self, src: Addr8) -> u8 {
    // TODO: Optimize
    let a = self.regs.a;
    let b = src.load(self);
    self.sub_(a, b, 0);
    self.regs.a = a;

    4 + src.cycles()
  }

  fn cpl(&mut self) -> u8 {
    self.regs.a = !self.regs.a;

    self.set_flag(ADD_SUB_FLAG, true);
    self.set_flag(HALF_CARRY_FLAG, true);

    4
  }

  fn scf(&mut self) -> u8 {
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, true);

    4
  }

  fn ccf(&mut self) -> u8 {
    self.regs.f ^= CARRY_FLAG;
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);

    4
  }

  fn daa(&mut self) -> u8 {
    // http://forums.nesdev.com/viewtopic.php?t=9088
    let mut a = self.regs.a as u16;

    if self.get_flag(ADD_SUB_FLAG) {
      if self.get_flag(HALF_CARRY_FLAG) {
        a = (a - 0x06) & 0xff;
      }
      if self.get_flag(CARRY_FLAG) {
        a -= 0x60;
      }
    } else {
      if self.get_flag(HALF_CARRY_FLAG) || (a & 0xf) > 9 {
        a += 0x06;
      }
      if self.get_flag(CARRY_FLAG) || a > 0x9f {
        a += 0x60;
      }
    }

    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(ZERO_FLAG, false);

    if (a & 0x100) != 0 {
      self.set_flag(CARRY_FLAG, true);
    }

    a &= 0xff;

    if a == 0 {
      self.set_flag(ZERO_FLAG, true);
    }

    self.regs.a = a as u8;

    4
  }

  //
  // Rotation/shift/bit
  //

  fn rlca(&mut self) -> u8 {
    self.rlc(Reg8Dir(A));

    self.set_flag(ZERO_FLAG, false);

    4
  }

  fn rla(&mut self) -> u8 {
    self.rl(Reg8Dir(A));

    self.set_flag(ZERO_FLAG, false);

    4
  }

  fn rrca(&mut self) -> u8 {
    self.rrc(Reg8Dir(A));

    self.set_flag(ZERO_FLAG, false);

    4
  }

  fn rra(&mut self) -> u8 {
    self.rr(Reg8Dir(A));

    self.set_flag(ZERO_FLAG, false);

    4
  }

  fn rlc(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    dst.store(self, (val << 1) | ((val & 0x80) >> 7));

    self.set_flag(ZERO_FLAG, val == 0); // zero iff zero before
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x80) != 0);

    8 + 2 * dst.cycles()
  }

  fn rl(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    let result = (val << 1) | ((self.regs.f & CARRY_FLAG) >> CARRY_OFFSET);
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x80) != 0);

    8 + 2 * dst.cycles()
  }

  fn rrc(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    dst.store(self, (val >> 1) | ((val & 0x01) << 7));

    self.set_flag(ZERO_FLAG, val == 0); // zero iff zero before
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x01) != 0);

    8 + 2 * dst.cycles()
  }

  fn rr(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    let result = (val >> 1) | ((self.regs.f & CARRY_FLAG) << (7 - CARRY_OFFSET));
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x01) != 0);

    8 + 2 * dst.cycles()
  }

  fn sla(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    let result = val << 1;
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x80) != 0);

    8 + 2 * dst.cycles()
  }

  fn sra(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    let result = (val & 0x80) | (val >> 1);
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x01) != 0);

    8 + 2 * dst.cycles()
  }

  fn srl(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    let result = val >> 1;
    dst.store(self, result);

    self.set_flag(ZERO_FLAG, result == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, (val & 0x01) != 0);

    8 + 2 * dst.cycles()
  }

  fn bit(&mut self, bit: u8, src: Addr8) -> u8 {
    let val = src.load(self);

    self.set_flag(ZERO_FLAG, (val & (1 << bit as uint)) == 0);
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, true);

    8 + src.cycles() // TODO: BIT b, (HL) takes 16 cycles
  }

  fn res(&mut self, bit: u8, dst: Addr8) -> u8 {
    let val = dst.load(self);

    dst.store(self, val & !(1 << bit as uint));

    8 + 2 * dst.cycles()
  }

  fn set(&mut self, bit: u8, dst: Addr8) -> u8 {
    let val = dst.load(self);

    dst.store(self, val | (1 << bit as uint));

    8 + 2 * dst.cycles()
  }

  fn swap(&mut self, dst: Addr8) -> u8 {
    let val = dst.load(self);
    dst.store(self, (val >> 4) | (val << 4));

    self.set_flag(ZERO_FLAG, val == 0); // zero iff zero before
    self.set_flag(ADD_SUB_FLAG, false);
    self.set_flag(HALF_CARRY_FLAG, false);
    self.set_flag(CARRY_FLAG, false);

    8 + 2 * dst.cycles()
  }

  // Undefined/illegal
  fn undef(&mut self, opcode: u8) -> u8 {
    panic!("illegal instruction: {:02X}", opcode)
  }
}
