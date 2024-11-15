use eframe::egui::{self,  ScrollArea, TextStyle, TextWrapMode, Ui, Vec2};
use holani::{mikey::cpu::M6502, Lynx};

macro_rules! get_word {
    ($lynx: ident, $addr: expr) => {
        ($lynx.cpu_mem($addr) as u16) | (($lynx.cpu_mem($addr+1) as u16) << 8)
    };
}

macro_rules! known_addr {
    ($addrs: ident, $addr: ident, $fnone: expr, $fvalue: expr) => {
        match &$addrs[$addr as usize] {
            None => format!($fnone, $addr),
            Some(v) => format!($fvalue, v),
        }
    };
}

#[derive(Default)]
struct DisasmToken {
    base_address: u16,
    data_length: u8,
    data: String,
    opcode: &'static str,
    operands: String,  
}

#[derive(Clone, Copy, PartialEq)]
enum AddressingMode
{
    Illegal,
    Accu,
    Imm,
    Absl,
    Zp,
    Zpx,
    Zpy,
    Absx,
    Absy,
    Iabsx,
    Implied,
    Rel,
    Zrel,
    Indx,
    Indy,
    Iabs,
    Ind
}

#[allow(dead_code)]
fn get_opcode_target(entry: &DisasmToken, cpu: &M6502, lynx: &Lynx) -> i16
{
    let mut addr = entry.base_address;
    let mut operand: u16 = 0;
    let count = next_address(lynx, addr) - addr;

    let opcode = lynx.cpu_mem(addr);

    match count {
        3 => {
            addr += 1;
            operand = lynx.cpu_mem(addr) as u16;
            addr += 1;
            operand += (lynx.cpu_mem(addr) as u16) << 8;
            addr += 1;
        }
        2 => {
            addr += 1;
            operand = lynx.cpu_mem(addr) as u16;
            addr += 1;
        }
        _ => {
            addr += 1;
        }
    }

    match INSTRUCTIONS[opcode as usize].1 {
        AddressingMode::Accu => -1,
        AddressingMode::Imm => -1,
        AddressingMode::Absl => lynx.cpu_mem(operand) as i16,
        AddressingMode::Rel => {
            let scrap: u16 = if operand > 128 {
                (-128 + (operand & 0x7f) as i16) as u16
            } else {
                operand
            };
            lynx.cpu_mem(addr + scrap) as i16
        }
        AddressingMode::Iabs => lynx.cpu_mem(get_word!(lynx, operand)) as i16,
        AddressingMode::Ind => lynx.cpu_mem(lynx.cpu_mem(operand) as u16) as i16,
        AddressingMode::Zp => lynx.cpu_mem(operand) as i16,
        AddressingMode::Zpx => lynx.cpu_mem(operand + cpu.x() as u16) as i16,
        AddressingMode::Zpy => lynx.cpu_mem(operand + cpu.y() as u16) as i16,
        AddressingMode::Absx => lynx.cpu_mem(operand + cpu.x() as u16) as i16,
        AddressingMode::Absy => lynx.cpu_mem(operand + cpu.y() as u16) as i16,
        AddressingMode::Iabsx => lynx.cpu_mem(get_word!(lynx, operand + cpu.x() as u16)) as i16,
        AddressingMode::Zrel => -1,
        AddressingMode::Indx => lynx.cpu_mem(get_word!(lynx, operand) + cpu.x() as u16) as i16,
        AddressingMode::Indy => lynx.cpu_mem(get_word!(lynx, operand) + cpu.y() as u16) as i16,
        AddressingMode::Implied => -1,
        AddressingMode::Illegal => -1,
    } 
}

fn disassemble(lynx: &Lynx, mut addr: u16, known: &[Option<String>]) -> (DisasmToken, u16) {
    let mut ret: DisasmToken = Default::default();

    let mut operand: u16 = 0;

    let count = match next_address(lynx, addr).checked_sub(addr)  {
        None => return (ret, addr.wrapping_add(1)),
        Some(v) => v,
    };

    ret.data_length = count as u8;
    ret.base_address = addr;

    let opcode = lynx.cpu_mem(addr);
    ret.data = format!("{:02X}", opcode);
    ret.opcode = INSTRUCTIONS[opcode as usize].0;

    match count {
        3 => {
            addr += 1;
            operand = lynx.cpu_mem(addr) as u16;
            addr += 1;
            operand += (lynx.cpu_mem(addr) as u16) << 8;
            addr += 1;
            ret.data += &format!(" {:02X} {:02X}", operand & 0xFF, operand >> 8).to_string();
        }
        2 => {
            addr += 1;
            operand = lynx.cpu_mem(addr) as u16;
            addr += 1;
            ret.data += &format!(" {:02X}", operand).to_string();
        }
        _ => {
            addr += 1;
        }
    }

    ret.operands = match INSTRUCTIONS[opcode as usize].1 {
        AddressingMode::Accu => "A".to_string(),
        AddressingMode::Imm => format!("#${:02X}", operand),
        AddressingMode::Absl => known_addr!(known, operand, "${:04X}", "{}"),
        AddressingMode::Rel => {
                let scrap: u16 = if operand > 128 {
                    (-128 + (operand & 0x7f) as i16) as u16
                } else {
                    operand
                };
                format!("${:04X}", addr.overflowing_add(scrap).0)
            }
        AddressingMode::Iabs => known_addr!(known, operand, "(${:04X})", "({})"),
        AddressingMode::Ind => format!("(${:02X})", operand),
        AddressingMode::Zp => format!("${:02X}", operand),
        AddressingMode::Zpx => format!("${:02X},X", operand),
        AddressingMode::Zpy => format!("${:02X},Y", operand),
        AddressingMode::Absx => known_addr!(known, operand, "${:04X},X", "{},X"),
        AddressingMode::Absy => known_addr!(known, operand, "${:04X},Y", "{},Y"),
        AddressingMode::Iabsx => known_addr!(known, operand, "(${:04X},X)", "({},X)"),
        AddressingMode::Zrel => {
                let mut scrap: u16 = operand >> 8;
                if scrap > 128 {
                    scrap = (-128 + (scrap & 0x7f) as i16) as u16;
                }
                format!("${:02X},${:04X}", operand & 0xff, addr.overflowing_add(scrap).0)
            }
        AddressingMode::Indx => format!("(${:02X}),X", operand),
        AddressingMode::Indy => format!("(${:02X}),Y", operand),
        AddressingMode::Implied => String::default(),
        AddressingMode::Illegal => String::default(),
    };

    (ret, addr)
}

fn next_address(lynx: &Lynx, addr: u16) -> u16 {
    let data: u8 = lynx.cpu_mem(addr);
    let operand = INSTRUCTIONS[data as usize].1;
    addr.overflowing_add(OP_LENGTH[operand as usize]).0
}

#[derive(Clone)]
struct DisasmWidgetOptions {
    pub text_style: TextStyle,
    pub is_options_collapsed: bool,
}

impl Default for DisasmWidgetOptions {
    fn default() -> Self {
        Self {     
            text_style: TextStyle::Monospace,   
            is_options_collapsed: false,
        }
    }
}

#[derive(Default)]
pub struct BetweenFrameData {
    pub previous_frame_editor_width: f32,
  }

pub struct DisasmWidget {
    options: DisasmWidgetOptions,
    visible_start_address: u16,
    frame_data: BetweenFrameData,
    known_addrs: Vec<Option<String>>
}

impl DisasmWidget {
    pub fn new() -> Self {
        const NONE: Option<String> = None;
        let mut s = Self {
            options: Default::default(),
            visible_start_address: 0,
            frame_data: Default::default(),
            known_addrs: vec![NONE; 0xffff+1]
        };
        s.initialize_known_addresses();
        s
    }

    fn draw_options_area(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("Options")
            .default_open(!self.options.is_options_collapsed)
            .show(ui, |ui| {
                    self.draw_main_options(ui);
            });
    }

    fn draw_main_options(&mut self, _ui: &mut Ui) {
    }

    pub fn disasm_show(&mut self, ui: &mut Ui, pc: u16, lynx: &Lynx) {
        self.draw_options_area(ui);

        ui.separator();

        let line_height = self.get_line_height(ui);        
        let max_lines = 0xffff;

        let scroll = ScrollArea::vertical()
            .max_height(f32::INFINITY)
            .auto_shrink([false, true]);

        let mut working_pc = pc;

        scroll.show_rows(ui, line_height, max_lines, |ui, line_range| {
            self.visible_start_address = line_range.start as u16;

            egui::Grid::new("mem_edit_grid")
                .striped(true)
                .spacing(Vec2::new(15.0, ui.style().spacing.item_spacing.y))
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    ui.style_mut().spacing.item_spacing.x = 3.0;

                    let mut current_line = line_range.start;
                    #[allow(unused_assignments)]
                    let mut token: DisasmToken = Default::default();

                    while current_line != line_range.end {
                        (token, working_pc) = disassemble(lynx, working_pc, &self.known_addrs);

                        ui.label(format!("{:04X}", token.base_address));
                        ui.label(token.data);
                        ui.label(token.opcode);
                        ui.label(token.operands);

                        ui.end_row();
                        current_line += 1;
                    }
                });
            self.frame_data.previous_frame_editor_width = ui.min_rect().width();
        });
    }

    fn get_line_height(&self, ui: &mut Ui) -> f32 {
        ui.text_style_height(&self.options.text_style)
    }

    fn initialize_known_addresses(&mut self) {
        self.known_addrs[0xFC00] = Some("TMPADRL".to_string());
        self.known_addrs[0xFC01] = Some("TMPADRH".to_string());
        self.known_addrs[0xFC02] = Some("TILTACUML".to_string());
        self.known_addrs[0xFC03] = Some("TILTACUMH".to_string());
        self.known_addrs[0xFC04] = Some("HOFFL".to_string());
        self.known_addrs[0xFC05] = Some("HOFFH".to_string());
        self.known_addrs[0xFC06] = Some("VOFFL".to_string());
        self.known_addrs[0xFC07] = Some("VOFFH".to_string());
        self.known_addrs[0xFC08] = Some("VIDBASL".to_string());
        self.known_addrs[0xFC09] = Some("VIDBASH".to_string());
        self.known_addrs[0xFC0A] = Some("COLLBASL".to_string());
        self.known_addrs[0xFC0B] = Some("COLLBASH".to_string());
        self.known_addrs[0xFC0C] = Some("VIDADRL".to_string());
        self.known_addrs[0xFC0D] = Some("VIDADRH".to_string());
        self.known_addrs[0xFC0E] = Some("COLLADRL".to_string());
        self.known_addrs[0xFC0F] = Some("COLLADRH".to_string());
        self.known_addrs[0xFC10] = Some("SCBNEXTL".to_string());
        self.known_addrs[0xFC11] = Some("SCBNEXTH".to_string());
        self.known_addrs[0xFC12] = Some("SPRDLINEL".to_string());
        self.known_addrs[0xFC13] = Some("SPRDLINEH".to_string());
        self.known_addrs[0xFC14] = Some("HPOSSTRTL".to_string());
        self.known_addrs[0xFC15] = Some("HPOSSTRTH".to_string());
        self.known_addrs[0xFC16] = Some("VPOSSTRTL".to_string());
        self.known_addrs[0xFC17] = Some("VPOSSTRTH".to_string());
        self.known_addrs[0xFC18] = Some("SPRHSIZL".to_string());
        self.known_addrs[0xFC19] = Some("SPRHSIZH".to_string());
        self.known_addrs[0xFC1A] = Some("SPRVSIZL".to_string());
        self.known_addrs[0xFC1B] = Some("SPRVSIZH".to_string());
        self.known_addrs[0xFC1C] = Some("STRETCHL".to_string());
        self.known_addrs[0xFC1D] = Some("STRETCHH".to_string());
        self.known_addrs[0xFC1E] = Some("TILTL".to_string());
        self.known_addrs[0xFC1F] = Some("TILTH".to_string());
        self.known_addrs[0xFC20] = Some("SPRDOFFL".to_string());
        self.known_addrs[0xFC21] = Some("SPRDOFFH".to_string());
        self.known_addrs[0xFC22] = Some("SPRVPOSL".to_string());
        self.known_addrs[0xFC23] = Some("SPRVPOSH".to_string());
        self.known_addrs[0xFC24] = Some("COLLOFFL".to_string());
        self.known_addrs[0xFC25] = Some("COLLOFFH".to_string());
        self.known_addrs[0xFC26] = Some("VSIZACUML".to_string());
        self.known_addrs[0xFC27] = Some("VSIZACUMH".to_string());
        self.known_addrs[0xFC28] = Some("HSIZOFFL".to_string());
        self.known_addrs[0xFC29] = Some("HSIZOFFH".to_string());
        self.known_addrs[0xFC2A] = Some("VSIZOFFL".to_string());
        self.known_addrs[0xFC2B] = Some("VSIZOFFH".to_string());
        self.known_addrs[0xFC2C] = Some("SCBADRL".to_string());
        self.known_addrs[0xFC2D] = Some("SCBADRH".to_string());
        self.known_addrs[0xFC2E] = Some("PROCADRL".to_string());
        self.known_addrs[0xFC2F] = Some("PROCADRH".to_string());
        self.known_addrs[0xFC52] = Some("MATHD".to_string());
        self.known_addrs[0xFC53] = Some("MATHC".to_string());
        self.known_addrs[0xFC54] = Some("MATHB".to_string());
        self.known_addrs[0xFC55] = Some("MATHA".to_string());
        self.known_addrs[0xFC56] = Some("MATHP".to_string());
        self.known_addrs[0xFC57] = Some("MATHN".to_string());
        self.known_addrs[0xFC60] = Some("MATHH".to_string());
        self.known_addrs[0xFC61] = Some("MATHG".to_string());
        self.known_addrs[0xFC62] = Some("MATHF".to_string());
        self.known_addrs[0xFC63] = Some("MATHE".to_string());
        self.known_addrs[0xFC6C] = Some("MATHM".to_string());
        self.known_addrs[0xFC6D] = Some("MATHL".to_string());
        self.known_addrs[0xFC6E] = Some("MATHK".to_string());
        self.known_addrs[0xFC6F] = Some("MATHJ".to_string());
        self.known_addrs[0xFC80] = Some("SPRCTL0".to_string());
        self.known_addrs[0xFC81] = Some("SPRCTL1".to_string());
        self.known_addrs[0xFC82] = Some("SPRCOLL".to_string());
        self.known_addrs[0xFC83] = Some("SPRINIT".to_string());
        self.known_addrs[0xFC88] = Some("SUZYHREV".to_string());
        self.known_addrs[0xFC89] = Some("SUZYSREV".to_string());
        self.known_addrs[0xFC90] = Some("SUZYBUSEN".to_string());
        self.known_addrs[0xFC91] = Some("SPRGO".to_string());
        self.known_addrs[0xFC92] = Some("SPRSYS".to_string());
        self.known_addrs[0xFCB0] = Some("JOYSTICK".to_string());
        self.known_addrs[0xFCB1] = Some("SWITCHES".to_string());
        self.known_addrs[0xFCB2] = Some("RCART0".to_string());
        self.known_addrs[0xFCB3] = Some("RCART1".to_string());
        self.known_addrs[0xFCC0] = Some("LEDS".to_string());
        self.known_addrs[0xFCC2] = Some("PARSTATUS".to_string());
        self.known_addrs[0xFCC3] = Some("PARDATA".to_string());
        self.known_addrs[0xFCC4] = Some("HOWIE".to_string());
        self.known_addrs[0xFD00] = Some("TIMER0".to_string());
        self.known_addrs[0xFD04] = Some("TIMER1".to_string());
        self.known_addrs[0xFD08] = Some("TIMER2".to_string());
        self.known_addrs[0xFD0C] = Some("TIMER3".to_string());
        self.known_addrs[0xFD10] = Some("TIMER4".to_string());
        self.known_addrs[0xFD14] = Some("TIMER5".to_string());
        self.known_addrs[0xFD18] = Some("TIMER6".to_string());
        self.known_addrs[0xFD1C] = Some("TIMER7".to_string());
        self.known_addrs[0xFD00] = Some("HTIMER".to_string());
        self.known_addrs[0xFD08] = Some("VTIMER".to_string());
        self.known_addrs[0xFD00] = Some("HTIMBKUP".to_string());
        self.known_addrs[0xFD01] = Some("HTIMCTLA".to_string());
        self.known_addrs[0xFD02] = Some("HTIMCNT".to_string());
        self.known_addrs[0xFD03] = Some("HTIMCTLB".to_string());
        self.known_addrs[0xFD08] = Some("VTIMBKUP".to_string());
        self.known_addrs[0xFD09] = Some("VTIMCTLA".to_string());
        self.known_addrs[0xFD0A] = Some("VTIMCNT".to_string());
        self.known_addrs[0xFD0B] = Some("VTIMCTLB".to_string());
        self.known_addrs[0xFD10] = Some("BAUDBKUP".to_string());
        self.known_addrs[0xFD00] = Some("TIM0BKUP".to_string());
        self.known_addrs[0xFD01] = Some("TIM0CTLA".to_string());
        self.known_addrs[0xFD02] = Some("TIM0CNT".to_string());
        self.known_addrs[0xFD03] = Some("TIM0CTLB".to_string());
        self.known_addrs[0xFD04] = Some("TIM1BKUP".to_string());
        self.known_addrs[0xFD05] = Some("TIM1CTLA".to_string());
        self.known_addrs[0xFD06] = Some("TIM1CNT".to_string());
        self.known_addrs[0xFD07] = Some("TIM1CTLB".to_string());
        self.known_addrs[0xFD08] = Some("TIM2BKUP".to_string());
        self.known_addrs[0xFD09] = Some("TIM2CTLA".to_string());
        self.known_addrs[0xFD0A] = Some("TIM2CNT".to_string());
        self.known_addrs[0xFD0B] = Some("TIM2CTLB".to_string());
        self.known_addrs[0xFD0C] = Some("TIM3BKUP".to_string());
        self.known_addrs[0xFD0D] = Some("TIM3CTLA".to_string());
        self.known_addrs[0xFD0E] = Some("TIM3CNT".to_string());
        self.known_addrs[0xFD0F] = Some("TIM3CTLB".to_string());
        self.known_addrs[0xFD10] = Some("TIM4BKUP".to_string());
        self.known_addrs[0xFD11] = Some("TIM4CTLA".to_string());
        self.known_addrs[0xFD12] = Some("TIM4CNT".to_string());
        self.known_addrs[0xFD13] = Some("TIM4CTLB".to_string());
        self.known_addrs[0xFD14] = Some("TIM5BKUP".to_string());
        self.known_addrs[0xFD15] = Some("TIM5CTLA".to_string());
        self.known_addrs[0xFD16] = Some("TIM5CNT".to_string());
        self.known_addrs[0xFD17] = Some("TIM5CTLB".to_string());
        self.known_addrs[0xFD18] = Some("TIM6BKUP".to_string());
        self.known_addrs[0xFD19] = Some("TIM6CTLA".to_string());
        self.known_addrs[0xFD1A] = Some("TIM6CNT".to_string());
        self.known_addrs[0xFD1B] = Some("TIM6CTLB".to_string());
        self.known_addrs[0xFD1C] = Some("TIM7BKUP".to_string());
        self.known_addrs[0xFD1D] = Some("TIM7CTLA".to_string());
        self.known_addrs[0xFD1E] = Some("TIM7CNT".to_string());
        self.known_addrs[0xFD1F] = Some("TIM7CTLB".to_string());
        self.known_addrs[0xFD20] = Some("AUDIO0".to_string());
        self.known_addrs[0xFD28] = Some("AUDIO1".to_string());
        self.known_addrs[0xFD30] = Some("AUDIO2".to_string());
        self.known_addrs[0xFD38] = Some("AUDIO3".to_string());
        self.known_addrs[0xFD20] = Some("AUD0VOL".to_string());
        self.known_addrs[0xFD21] = Some("AUD0FEED".to_string());
        self.known_addrs[0xFD22] = Some("AUD0OUT".to_string());
        self.known_addrs[0xFD23] = Some("AUD0SHIFT".to_string());
        self.known_addrs[0xFD24] = Some("AUD0BKUP".to_string());
        self.known_addrs[0xFD25] = Some("AUD0CTLA".to_string());
        self.known_addrs[0xFD26] = Some("AUD0CNT".to_string());
        self.known_addrs[0xFD27] = Some("AUD0CTLB".to_string());
        self.known_addrs[0xFD28] = Some("AUD1VOL".to_string());
        self.known_addrs[0xFD29] = Some("AUD1FEED".to_string());
        self.known_addrs[0xFD2A] = Some("AUD1OUT".to_string());
        self.known_addrs[0xFD2B] = Some("AUD1SHIFT".to_string());
        self.known_addrs[0xFD2C] = Some("AUD1BKUP".to_string());
        self.known_addrs[0xFD2D] = Some("AUD1CTLA".to_string());
        self.known_addrs[0xFD2E] = Some("AUD1CNT".to_string());
        self.known_addrs[0xFD2F] = Some("AUD1CTLB".to_string());
        self.known_addrs[0xFD30] = Some("AUD2VOL".to_string());
        self.known_addrs[0xFD31] = Some("AUD2FEED".to_string());
        self.known_addrs[0xFD32] = Some("AUD2OUT".to_string());
        self.known_addrs[0xFD33] = Some("AUD2SHIFT".to_string());
        self.known_addrs[0xFD34] = Some("AUD2BKUP".to_string());
        self.known_addrs[0xFD35] = Some("AUD2CTLA".to_string());
        self.known_addrs[0xFD36] = Some("AUD2CNT".to_string());
        self.known_addrs[0xFD37] = Some("AUD2CTLB".to_string());
        self.known_addrs[0xFD38] = Some("AUD3VOL".to_string());
        self.known_addrs[0xFD39] = Some("AUD3FEED".to_string());
        self.known_addrs[0xFD3A] = Some("AUD3OUT".to_string());
        self.known_addrs[0xFD3B] = Some("AUD3SHIFT".to_string());
        self.known_addrs[0xFD3C] = Some("AUD3BKUP".to_string());
        self.known_addrs[0xFD3D] = Some("AUD3CTLA".to_string());
        self.known_addrs[0xFD3E] = Some("AUD3CNT".to_string());
        self.known_addrs[0xFD3F] = Some("AUD3CTLB".to_string());
        self.known_addrs[0xFD50] = Some("MSTEREO".to_string());
        self.known_addrs[0xFD80] = Some("INTRST".to_string());
        self.known_addrs[0xFD81] = Some("INTSET".to_string());
        self.known_addrs[0xFD84] = Some("MAGRDY0".to_string());
        self.known_addrs[0xFD85] = Some("MAGRDY1".to_string());
        self.known_addrs[0xFD86] = Some("AUDIN".to_string());
        self.known_addrs[0xFD87] = Some("SYSCTL1".to_string());
        self.known_addrs[0xFD88] = Some("MIKEYHREV".to_string());
        self.known_addrs[0xFD89] = Some("MIKEYSREV".to_string());
        self.known_addrs[0xFD8A] = Some("IODIR".to_string());
        self.known_addrs[0xFD8B] = Some("IODAT".to_string());
        self.known_addrs[0xFD8C] = Some("SERCTL".to_string());
        self.known_addrs[0xFD8D] = Some("SERDAT".to_string());
        self.known_addrs[0xFD90] = Some("SDONEACK".to_string());
        self.known_addrs[0xFD91] = Some("CPUSLEEP".to_string());
        self.known_addrs[0xFD92] = Some("DISPCTL".to_string());
        self.known_addrs[0xFD93] = Some("PBKUP".to_string());
        self.known_addrs[0xFD94] = Some("DISPADRL".to_string());
        self.known_addrs[0xFD95] = Some("DISPADRH".to_string());
        self.known_addrs[0xFD9C] = Some("MTEST0".to_string());
        self.known_addrs[0xFD9D] = Some("MTEST1".to_string());
        self.known_addrs[0xFD9E] = Some("MTEST2".to_string());
        self.known_addrs[0xFDA0] = Some("PALETTE".to_string());
        self.known_addrs[0xFDA0] = Some("GCOLMAP".to_string());
        self.known_addrs[0xFDB0] = Some("RBCOLMAP".to_string());
        self.known_addrs[0xFFF9] = Some("MAPCTL".to_string());
        self.known_addrs[0xFFFB] = Some("VECTORS".to_string());
        self.known_addrs[0xFFFE] = Some("INTVECTL".to_string());
        self.known_addrs[0xFFFF] = Some("INTVECTH".to_string());
        self.known_addrs[0xFFFC] = Some("RSTVECTL".to_string());
        self.known_addrs[0xFFFD] = Some("RSTVECTH".to_string());
        self.known_addrs[0xFFFA] = Some("NMIVECTL".to_string());
        self.known_addrs[0xFFFB] = Some("NMIVECTH".to_string());
    }
}

// void DisasmEditor::scroll_down()
// {
//     _local_pc = next_address(_local_pc);
// }

// void DisasmEditor::scroll_up()
// {
//     uint8_t data;
//     int operand, size;
//     uint16_t address = _local_pc;

//     if (address > 0xffffU)
//         address = 0xffffU;
//     if (address < 4)
//     {
//         _local_pc = address;
//         return;
//     }

//     for (int loop = 1; loop < 4; loop++)
//     {
//         data = cpu_mem(address - loop);
//         operand = mLookupTable[data].mode;
//         size = mOperandSizes[operand];

//         if (size == loop)
//         {
//             _local_pc = ((address - loop) & 0xffff);
//             return;
//         }
//     }

//     _local_pc = ((address - 1) & 0xffff);
// }
const OP_LENGTH: [u16; 17] = [1, 1, 2, 3, 2, 2, 2, 3, 3, 3, 1, 2, 3, 2, 2, 3, 2];

const INSTRUCTIONS: [(&str, AddressingMode); 0x100] = [
   // 0x00
   ("BRK", AddressingMode::Implied),
   ("ORA", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("TSB", AddressingMode::Zp),
   ("ORA", AddressingMode::Zp),
   ("ASL", AddressingMode::Zp),
   ("RMB0", AddressingMode::Zp),
   ("PHP", AddressingMode::Implied),
   ("ORA", AddressingMode::Imm),
   ("ASL", AddressingMode::Accu),
   ("NOP11", AddressingMode::Illegal),
   ("TSB", AddressingMode::Absl),
   ("ORA", AddressingMode::Absl),
   ("ASL", AddressingMode::Absl),
   ("BBR0", AddressingMode::Zrel),
   // 0x10
   ("BPL", AddressingMode::Rel),
   ("ORA", AddressingMode::Indy),
   ("ORA", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("TRB", AddressingMode::Zp),
   ("ORA", AddressingMode::Zpx),
   ("ASL", AddressingMode::Zpx),
   ("RMB1", AddressingMode::Zp),
   ("CLC", AddressingMode::Implied),
   ("ORA", AddressingMode::Absy),
   ("INC", AddressingMode::Accu),
   ("NOP11", AddressingMode::Illegal),
   ("TRB", AddressingMode::Absl),
   ("ORA", AddressingMode::Absx),
   ("ASL", AddressingMode::Absx),
   ("BBR1", AddressingMode::Zrel),
   // 0x20
   ("JSR", AddressingMode::Absl),
   ("AND", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("BIT", AddressingMode::Zp),
   ("AND", AddressingMode::Zp),
   ("ROL", AddressingMode::Zp),
   ("RMB2", AddressingMode::Zp),
   ("PLP", AddressingMode::Implied),
   ("AND", AddressingMode::Imm),
   ("ROL", AddressingMode::Accu),
   ("NOP11", AddressingMode::Illegal),
   ("BIT", AddressingMode::Absl),
   ("AND", AddressingMode::Absl),
   ("ROL", AddressingMode::Absl),
   ("BBR2", AddressingMode::Zrel),
   // 0x30
   ("BMI", AddressingMode::Rel),
   ("AND", AddressingMode::Indy),
   ("AND", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("BIT", AddressingMode::Zpx),
   ("AND", AddressingMode::Zpx),
   ("ROL", AddressingMode::Zpx),
   ("RMB3", AddressingMode::Zp),
   ("SEC", AddressingMode::Implied),
   ("AND", AddressingMode::Absy),
   ("DEC", AddressingMode::Accu),
   ("NOP11", AddressingMode::Illegal),
   ("BIT", AddressingMode::Absx),
   ("AND", AddressingMode::Absx),
   ("ROL", AddressingMode::Absx),
   ("BBR3", AddressingMode::Zrel),
   // 0x40
   ("RTI", AddressingMode::Implied),
   ("EOR", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("NOP23", AddressingMode::Illegal),
   ("EOR", AddressingMode::Zp),
   ("LSR", AddressingMode::Zp),
   ("RMB4", AddressingMode::Zp),
   ("PHA", AddressingMode::Implied),
   ("EOR", AddressingMode::Imm),
   ("LSR", AddressingMode::Accu),
   ("NOP11", AddressingMode::Illegal),
   ("JMP", AddressingMode::Absl),
   ("EOR", AddressingMode::Absl),
   ("LSR", AddressingMode::Absl),
   ("BBR4", AddressingMode::Zrel),
   // 0x50
   ("BVC", AddressingMode::Rel),
   ("EOR", AddressingMode::Indy),
   ("EOR", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("NOP23", AddressingMode::Illegal),
   ("EOR", AddressingMode::Zpx),
   ("LSR", AddressingMode::Zpx),
   ("RMB5", AddressingMode::Zp),
   ("CLI", AddressingMode::Implied),
   ("EOR", AddressingMode::Absy),
   ("PHY", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("NOP38", AddressingMode::Illegal),
   ("EOR", AddressingMode::Absx),
   ("LSR", AddressingMode::Absx),
   ("BBR5", AddressingMode::Zrel),
   // 0x60
   ("RTS", AddressingMode::Implied),
   ("ADC", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("STZ", AddressingMode::Zp),
   ("ADC", AddressingMode::Zp),
   ("ROR", AddressingMode::Zp),
   ("RMB6", AddressingMode::Zp),
   ("PLA", AddressingMode::Implied),
   ("ADC", AddressingMode::Imm),
   ("ROR", AddressingMode::Accu),
   ("NOP11", AddressingMode::Illegal),
   ("JMP", AddressingMode::Iabs),
   ("ADC", AddressingMode::Absl),
   ("ROR", AddressingMode::Absl),
   ("BBR6", AddressingMode::Zrel),
   // 0x70
   ("BVS", AddressingMode::Rel),
   ("ADC", AddressingMode::Indy),
   ("ADC", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("STZ", AddressingMode::Zpx),
   ("ADC", AddressingMode::Zpx),
   ("ROR", AddressingMode::Zpx),
   ("RMB7", AddressingMode::Zp),
   ("SEI", AddressingMode::Implied),
   ("ADC", AddressingMode::Absy),
   ("PLY", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("JMP", AddressingMode::Iabsx),
   ("ADC", AddressingMode::Absx),
   ("ROR", AddressingMode::Absx),
   ("BBR7", AddressingMode::Zrel),
   // 0x80
   ("BRA", AddressingMode::Rel),
   ("STA", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("STY", AddressingMode::Zp),
   ("STA", AddressingMode::Zp),
   ("STX", AddressingMode::Zp),
   ("SMB0", AddressingMode::Zp),
   ("DEY", AddressingMode::Implied),
   ("BIT", AddressingMode::Imm),
   ("TXA", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("STY", AddressingMode::Absl),
   ("STA", AddressingMode::Absl),
   ("STX", AddressingMode::Absl),
   ("BBS0", AddressingMode::Zrel),
   // 0x90
   ("BCC", AddressingMode::Rel),
   ("STA", AddressingMode::Indy),
   ("STA", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("STY", AddressingMode::Zpx),
   ("STA", AddressingMode::Zpx),
   ("STX", AddressingMode::Zpy),
   ("SMB1", AddressingMode::Zp),
   ("TYA", AddressingMode::Implied),
   ("STA", AddressingMode::Absy),
   ("TXS", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("STZ", AddressingMode::Absl),
   ("STA", AddressingMode::Absx),
   ("STZ", AddressingMode::Absx),
   ("BBS1", AddressingMode::Zrel),
   // 0xA0
   ("LDY", AddressingMode::Imm),
   ("LDA", AddressingMode::Indx),
   ("LDX", AddressingMode::Imm),
   ("NOP11", AddressingMode::Illegal),
   ("LDY", AddressingMode::Zp),
   ("LDA", AddressingMode::Zp),
   ("LDX", AddressingMode::Zp),
   ("SMB2", AddressingMode::Zp),
   ("TAY", AddressingMode::Implied),
   ("LDA", AddressingMode::Imm),
   ("TAX", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("LDY", AddressingMode::Absl),
   ("LDA", AddressingMode::Absl),
   ("LDX", AddressingMode::Absl),
   ("BBS2", AddressingMode::Zrel),
   // 0xB0
   ("BCS", AddressingMode::Rel),
   ("LDA", AddressingMode::Indy),
   ("LDA", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("LDY", AddressingMode::Zpx),
   ("LDA", AddressingMode::Zpx),
   ("LDX", AddressingMode::Zpy),
   ("SMB3", AddressingMode::Zp),
   ("CLV", AddressingMode::Implied),
   ("LDA", AddressingMode::Absy),
   ("TSX", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("LDY", AddressingMode::Absx),
   ("LDA", AddressingMode::Absx),
   ("LDX", AddressingMode::Absy),
   ("BBS3", AddressingMode::Zrel),
   // 0xC0
   ("CPY", AddressingMode::Imm),
   ("CMP", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("CPY", AddressingMode::Zp),
   ("CMP", AddressingMode::Zp),
   ("DEC", AddressingMode::Zp),
   ("SMB4", AddressingMode::Zp),
   ("INY", AddressingMode::Implied),
   ("CMP", AddressingMode::Imm),
   ("DEX", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("CPY", AddressingMode::Absl),
   ("CMP", AddressingMode::Absl),
   ("DEC", AddressingMode::Absl),
   ("BBS4", AddressingMode::Zrel),
   // 0xD0
   ("BNE", AddressingMode::Rel),
   ("CMP", AddressingMode::Indy),
   ("CMP", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("NOP24", AddressingMode::Illegal),
   ("CMP", AddressingMode::Zpx),
   ("DEC", AddressingMode::Zpx),
   ("SMB5", AddressingMode::Zp),
   ("CLD", AddressingMode::Implied),
   ("CMP", AddressingMode::Absy),
   ("PHX", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("NOP34", AddressingMode::Illegal),
   ("CMP", AddressingMode::Absx),
   ("DEC", AddressingMode::Absx),
   ("BBS5", AddressingMode::Zrel),
   // 0xE0
   ("CPX", AddressingMode::Imm),
   ("SBC", AddressingMode::Indx),
   ("NOP22", AddressingMode::Illegal),
   ("NOP11", AddressingMode::Illegal),
   ("CPX", AddressingMode::Zp),
   ("SBC", AddressingMode::Zp),
   ("INC", AddressingMode::Zp),
   ("SMB6", AddressingMode::Zp),
   ("INX", AddressingMode::Implied),
   ("SBC", AddressingMode::Imm),
   ("NOP", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("CPX", AddressingMode::Absl),
   ("SBC", AddressingMode::Absl),
   ("INC", AddressingMode::Absl),
   ("BBS6", AddressingMode::Zrel),
   // 0xF0
   ("BEQ", AddressingMode::Rel),
   ("SBC", AddressingMode::Indy),
   ("SBC", AddressingMode::Ind),
   ("NOP11", AddressingMode::Illegal),
   ("NOP24", AddressingMode::Illegal),
   ("SBC", AddressingMode::Zpx),
   ("INC", AddressingMode::Zpx),
   ("SMB7", AddressingMode::Zp),
   ("SED", AddressingMode::Implied),
   ("SBC", AddressingMode::Absy),
   ("PLX", AddressingMode::Implied),
   ("NOP11", AddressingMode::Illegal),
   ("NOP34", AddressingMode::Illegal),
   ("SBC", AddressingMode::Absx),
   ("INC", AddressingMode::Absx),
   ("BBS7", AddressingMode::Zrel),
];