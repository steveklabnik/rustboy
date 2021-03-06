use cpu::{Addr8, Addr16, Cond, Decoder};
use cpu;
use mem;

pub struct Disasm<'a, M:'a> {
  pub mem: &'a mut M,
  pub pc: u16,
}

fn addr16_to_str(addr: Addr16) -> String {
  match addr {
    cpu::Imm16(val)    => format!("${:04X}", val),
    cpu::Ind16(addr)   => format!("(${:04X})", addr),
    cpu::Reg16Dir(r)   => format!("{}", r),
  }
}

fn addr8_to_str(addr: Addr8) -> String {
  match addr {
    cpu::Imm8(val)       => format!("${:02X}", val),
    cpu::Ind8(offset)    => format!("($FF00+${:02X})", offset),
    cpu::Imm16Ind8(addr) => format!("(${:04X})", addr),
    cpu::Reg8Dir(r)      => format!("{}", r),
    cpu::Reg8Ind8(r)     => format!("($FF00+{})", r),
    cpu::Reg16Ind8(r)    => format!("({})", r),
    cpu::Reg16Ind8Inc(r) => format!("({}+)", r),
    cpu::Reg16Ind8Dec(r) => format!("({}-)", r),
  }
}

fn cond_to_str(cond: Cond) -> &'static str {
  match cond {
    cpu::CondNone => "",
    cpu::CondZ    => "Z",
    cpu::CondNZ   => "NZ",
    cpu::CondC    => "C",
    cpu::CondNC   => "NC",
  }
}

fn with_cond(mnemonic: &str, cond: Cond) -> String {
  match cond {
    cpu::CondNone => mnemonic.to_string(),
    _             => format!("{:s} {:s},", mnemonic, cond_to_str(cond)),
  }
}

fn unary8(mnemonic: &str, addr: Addr8) -> String {
  format!("{:s} {:s}", mnemonic, addr8_to_str(addr))
}

fn binary8(mnemonic: &str, addr1: Addr8, addr2: Addr8) -> String {
  format!("{:s} {:s}, {:s}", mnemonic, addr8_to_str(addr1), addr8_to_str(addr2))
}

fn unary16(mnemonic: &str, addr: Addr16) -> String {
  format!("{:s} {:s}", mnemonic, addr16_to_str(addr))
}

fn binary16(mnemonic: &str, addr1: Addr16, addr2: Addr16) -> String {
  format!("{:s} {:s}, {:s}", mnemonic, addr16_to_str(addr1), addr16_to_str(addr2))
}

impl<'a, M: mem::Mem> Decoder<String> for Disasm<'a, M> {
  fn fetch(&mut self) -> u8 {
    let result = self.mem.loadb(self.pc);
    self.pc += 1;
    result
  }

  // Misc/control
  fn nop (&mut self)          -> String { "NOP".to_string() }

  fn ei  (&mut self)          -> String { "EI".to_string() }
  fn di  (&mut self)          -> String { "DI".to_string() }

  fn halt(&mut self)          -> String { "HALT".to_string() }
  fn stop(&mut self, val: u8) -> String { format!("STOP {:u}", val) }

  // Jump/call
  fn jp  (&mut self, cond: Cond, addr: Addr16) -> String { unary16(with_cond("JP", cond).as_slice(), addr) }
  fn jr  (&mut self, cond: Cond, rel: i8)      -> String { format!("{:s} {:d}", with_cond("JR", cond), rel) }

  fn call(&mut self, cond: Cond, addr: Addr16) -> String { unary16(with_cond("CALL", cond).as_slice(), addr) }
  fn rst (&mut self, addr: u8)                 -> String { format!("RST ${:02X}", addr) }

  fn ret (&mut self, cond: Cond)               -> String { format!("RET {:s}", cond_to_str(cond)) }
  fn reti(&mut self)                           -> String {         "RETI".to_string() }

  // Load/store/move
  fn ld8 (&mut self, dst: Addr8,  src: Addr8)  -> String { binary8 ("LD", dst, src) }
  fn ld16(&mut self, dst: Addr16, src: Addr16) -> String { binary16("LD", dst, src) }
  fn ldh (&mut self, dst: Addr8,  src: Addr8)  -> String { binary8 ("LDH", dst, src) }
  fn ldhl(&mut self, rel: i8)                  -> String { format! ("LDHL SP, {:d}", rel) }

  fn push(&mut self, src: Addr16)              -> String { unary16 ("PUSH", src) }
  fn pop (&mut self, dst: Addr16)              -> String { unary16 ("POP", dst) }

  // Arithmetic/logic
  fn add8 (&mut self, src: Addr8)               -> String { unary8  ("ADD A,", src) }
  fn add16(&mut self, dst: Addr16, src: Addr16) -> String { binary16("ADD", dst, src) }
  fn addsp(&mut self, rel: i8)                  -> String { format! ("ADD SP, {:d}", rel) }
  fn adc  (&mut self, src: Addr8)               -> String { unary8  ("ADC A,", src) }

  fn sub  (&mut self, src: Addr8)               -> String { unary8  ("SUB", src) }
  fn sbc  (&mut self, src: Addr8)               -> String { unary8  ("SBC A,", src) }

  fn inc8 (&mut self, dst: Addr8)               -> String { unary8  ("INC", dst) }
  fn inc16(&mut self, dst: Addr16)              -> String { unary16 ("INC", dst) }
  fn dec8 (&mut self, dst: Addr8)               -> String { unary8  ("DEC", dst) }
  fn dec16(&mut self, dst: Addr16)              -> String { unary16 ("DEC", dst) }

  fn and  (&mut self, src: Addr8)               -> String { unary8  ("AND", src) }
  fn or   (&mut self, src: Addr8)               -> String { unary8  ("OR", src) }
  fn xor  (&mut self, src: Addr8)               -> String { unary8  ("XOR", src) }

  fn cp   (&mut self, src: Addr8)               -> String { unary8  ("CP", src) }

  fn cpl  (&mut self)                           -> String {          "CPL".to_string() }

  fn scf  (&mut self)                           -> String {          "SCF".to_string() }
  fn ccf  (&mut self)                           -> String {          "CCF".to_string() }

  fn daa  (&mut self)                           -> String {          "DAA".to_string() }

  // Rotation/shift/bit
  fn rlca(&mut self)                      -> String {         "RLCA".to_string() }
  fn rla (&mut self)                      -> String {         "RLA".to_string() }
  fn rrca(&mut self)                      -> String {         "RRCA".to_string() }
  fn rra (&mut self)                      -> String {         "RRA".to_string() }

  fn rlc (&mut self, dst: Addr8)          -> String { unary8 ("RLC", dst) }
  fn rl  (&mut self, dst: Addr8)          -> String { unary8 ("RL", dst) }
  fn rrc (&mut self, dst: Addr8)          -> String { unary8 ("RRC", dst) }
  fn rr  (&mut self, dst: Addr8)          -> String { unary8 ("RR", dst) }

  fn sla (&mut self, dst: Addr8)          -> String { unary8 ("SLA", dst) }
  fn sra (&mut self, dst: Addr8)          -> String { unary8 ("SRA", dst) }
  fn srl (&mut self, dst: Addr8)          -> String { unary8 ("SRL", dst) }

  fn bit (&mut self, bit: u8, src: Addr8) -> String { format!("BIT {:u}, {:s}", bit, addr8_to_str(src)) }
  fn res (&mut self, bit: u8, dst: Addr8) -> String { format!("RES {:u}, {:s}", bit, addr8_to_str(dst)) }
  fn set (&mut self, bit: u8, dst: Addr8) -> String { format!("SET {:u}, {:s}", bit, addr8_to_str(dst)) }

  fn swap(&mut self, dst: Addr8)          -> String { unary8 ("SWAP", dst) }

  // Undefined/illegal
  fn undef(&mut self, opcode: u8) -> String { format!("UNDEFINED ${:02X}", opcode) }
}
