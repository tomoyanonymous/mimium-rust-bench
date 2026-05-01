/* ------------------------------------------------------------
name: "osc"
Code generated with Faust 2.81.10 (https://faust.grame.fr)
Compilation options: -a /usr/local/share/faust/rust/portaudio-float.rs -lang rust -ct 1 -es 1 -mcd 16 -mdd 1024 -mdy 33 -single -ftz 0
------------------------------------------------------------ */
/************************************************************************
 FAUST Architecture File
 Copyright (C) 2003-2024 GRAME, Centre National de Creation Musicale
 ---------------------------------------------------------------------
 This Architecture section is free software; you can redistribute it
 and/or modify it under the terms of the GNU General Public License
 as published by the Free Software Foundation; either version 3 of
 the License, or (at your option) any later version.
 
 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.
 
 You should have received a copy of the GNU General Public License
 along with this program; If not, see <http://www.gnu.org/licenses/>.
 
 EXCEPTION : As a special exception, you may create a larger work
 that contains this FAUST architecture section and distribute
 that work under terms of your choice, so long as this FAUST
 architecture section is not modified.
 
 ************************************************************************
 ************************************************************************/

#![allow(unused_parens)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(non_upper_case_globals)]

//! PortAudio architecture file

use std::io;
extern crate libm;

type F32 = f32;
type F64 = f64;

#[derive(Copy, Clone)]
pub struct ParamIndex(pub i32);

pub struct Soundfile<'a,T> {
    fBuffers: &'a&'a T,
    fLength: &'a i32,
    fSR: &'a i32,
    fOffset: &'a i32,
    fChannels: i32
}

pub trait FaustDsp {
    type T;

    fn new() -> Self where Self: Sized;
    fn metadata(&self, m: &mut dyn Meta);
    fn get_sample_rate(&self) -> i32;
    fn get_num_inputs(&self) -> i32;
    fn get_num_outputs(&self) -> i32;
    fn class_init(sample_rate: i32) where Self: Sized;
    fn instance_reset_params(&mut self);
    fn instance_clear(&mut self);
    fn instance_constants(&mut self, sample_rate: i32);
    fn instance_init(&mut self, sample_rate: i32);
    fn init(&mut self, sample_rate: i32);
    fn build_user_interface(&self, ui_interface: &mut dyn UI<Self::T>);
    fn build_user_interface_static(ui_interface: &mut dyn UI<Self::T>) where Self: Sized;
    fn get_param(&self, param: ParamIndex) -> Option<Self::T>;
    fn set_param(&mut self, param: ParamIndex, value: Self::T);
    fn compute(&mut self, count: i32, inputs: &[&[Self::T]], outputs: &mut[&mut[Self::T]]);
}

pub trait Meta {
    // -- metadata declarations
    fn declare(&mut self, key: &str, value: &str);
}

pub trait UI<T> {
    // -- widget's layouts
    fn open_tab_box(&mut self, label: &str);
    fn open_horizontal_box(&mut self, label: &str);
    fn open_vertical_box(&mut self, label: &str);
    fn close_box(&mut self);

    // -- active widgets
    fn add_button(&mut self, label: &str, param: ParamIndex);
    fn add_check_button(&mut self, label: &str, param: ParamIndex);
    fn add_vertical_slider(&mut self, label: &str, param: ParamIndex, init: T, min: T, max: T, step: T);
    fn add_horizontal_slider(&mut self, label: &str, param: ParamIndex, init: T, min: T, max: T, step: T);
    fn add_num_entry(&mut self, label: &str, param: ParamIndex, init: T, min: T, max: T, step: T);

    // -- passive widgets
    fn add_horizontal_bargraph(&mut self, label: &str, param: ParamIndex, min: T, max: T);
    fn add_vertical_bargraph(&mut self, label: &str, param: ParamIndex, min: T, max: T);

    // -- metadata declarations
    fn declare(&mut self, param: Option<ParamIndex>, key: &str, value: &str);
}

/* ------------------------------------------------------------
name: "replicate100"
Code generated with Faust 2.83.1 (https://faust.grame.fr)
Compilation options: -lang rust -fpga-mem-th 4 -ct 1 -es 1 -mcd 16 -mdd 1024 -mdy 33 -single -ftz 0
------------------------------------------------------------ */
#[cfg_attr(feature = "default-boxed", derive(default_boxed::DefaultBoxed))]
#[repr(C)]
pub struct mydsp {
	iVec0: [i32;2],
	fRec0: [F32;2],
	fSampleRate: i32,
	fConst0: F32,
	fConst1: F32,
	fRec1: [F32;2],
	fConst2: F32,
	fRec2: [F32;2],
	fConst3: F32,
	fRec3: [F32;2],
	fConst4: F32,
	fRec4: [F32;2],
	fConst5: F32,
	fRec5: [F32;2],
	fConst6: F32,
	fRec6: [F32;2],
	fConst7: F32,
	fRec7: [F32;2],
	fConst8: F32,
	fRec8: [F32;2],
	fConst9: F32,
	fRec9: [F32;2],
	fConst10: F32,
	fRec10: [F32;2],
	fConst11: F32,
	fRec11: [F32;2],
	fConst12: F32,
	fRec12: [F32;2],
	fConst13: F32,
	fRec13: [F32;2],
	fConst14: F32,
	fRec14: [F32;2],
	fConst15: F32,
	fRec15: [F32;2],
	fConst16: F32,
	fRec16: [F32;2],
	fConst17: F32,
	fRec17: [F32;2],
	fConst18: F32,
	fRec18: [F32;2],
	fConst19: F32,
	fRec19: [F32;2],
	fConst20: F32,
	fRec20: [F32;2],
	fConst21: F32,
	fRec21: [F32;2],
	fConst22: F32,
	fRec22: [F32;2],
	fConst23: F32,
	fRec23: [F32;2],
	fConst24: F32,
	fRec24: [F32;2],
	fConst25: F32,
	fRec25: [F32;2],
	fConst26: F32,
	fRec26: [F32;2],
	fConst27: F32,
	fRec27: [F32;2],
	fConst28: F32,
	fRec28: [F32;2],
	fConst29: F32,
	fRec29: [F32;2],
	fConst30: F32,
	fRec30: [F32;2],
	fConst31: F32,
	fRec31: [F32;2],
	fConst32: F32,
	fRec32: [F32;2],
	fConst33: F32,
	fRec33: [F32;2],
	fConst34: F32,
	fRec34: [F32;2],
	fConst35: F32,
	fRec35: [F32;2],
	fConst36: F32,
	fRec36: [F32;2],
	fConst37: F32,
	fRec37: [F32;2],
	fConst38: F32,
	fRec38: [F32;2],
	fConst39: F32,
	fRec39: [F32;2],
	fConst40: F32,
	fRec40: [F32;2],
	fConst41: F32,
	fRec41: [F32;2],
	fConst42: F32,
	fRec42: [F32;2],
	fConst43: F32,
	fRec43: [F32;2],
	fConst44: F32,
	fRec44: [F32;2],
	fConst45: F32,
	fRec45: [F32;2],
	fConst46: F32,
	fRec46: [F32;2],
	fConst47: F32,
	fRec47: [F32;2],
	fConst48: F32,
	fRec48: [F32;2],
	fConst49: F32,
	fRec49: [F32;2],
	fConst50: F32,
	fRec50: [F32;2],
	fConst51: F32,
	fRec51: [F32;2],
	fConst52: F32,
	fRec52: [F32;2],
	fConst53: F32,
	fRec53: [F32;2],
	fConst54: F32,
	fRec54: [F32;2],
	fConst55: F32,
	fRec55: [F32;2],
	fConst56: F32,
	fRec56: [F32;2],
	fConst57: F32,
	fRec57: [F32;2],
	fConst58: F32,
	fRec58: [F32;2],
	fConst59: F32,
	fRec59: [F32;2],
	fConst60: F32,
	fRec60: [F32;2],
	fConst61: F32,
	fRec61: [F32;2],
	fConst62: F32,
	fRec62: [F32;2],
	fConst63: F32,
	fRec63: [F32;2],
	fConst64: F32,
	fRec64: [F32;2],
	fConst65: F32,
	fRec65: [F32;2],
	fConst66: F32,
	fRec66: [F32;2],
	fConst67: F32,
	fRec67: [F32;2],
	fConst68: F32,
	fRec68: [F32;2],
	fConst69: F32,
	fRec69: [F32;2],
	fConst70: F32,
	fRec70: [F32;2],
	fConst71: F32,
	fRec71: [F32;2],
	fConst72: F32,
	fRec72: [F32;2],
	fConst73: F32,
	fRec73: [F32;2],
	fConst74: F32,
	fRec74: [F32;2],
	fConst75: F32,
	fRec75: [F32;2],
	fConst76: F32,
	fRec76: [F32;2],
	fConst77: F32,
	fRec77: [F32;2],
	fConst78: F32,
	fRec78: [F32;2],
	fConst79: F32,
	fRec79: [F32;2],
	fConst80: F32,
	fRec80: [F32;2],
	fConst81: F32,
	fRec81: [F32;2],
	fConst82: F32,
	fRec82: [F32;2],
	fConst83: F32,
	fRec83: [F32;2],
	fConst84: F32,
	fRec84: [F32;2],
	fConst85: F32,
	fRec85: [F32;2],
	fConst86: F32,
	fRec86: [F32;2],
	fConst87: F32,
	fRec87: [F32;2],
	fConst88: F32,
	fRec88: [F32;2],
	fConst89: F32,
	fRec89: [F32;2],
	fConst90: F32,
	fRec90: [F32;2],
	fConst91: F32,
	fRec91: [F32;2],
	fConst92: F32,
	fRec92: [F32;2],
	fConst93: F32,
	fRec93: [F32;2],
	fConst94: F32,
	fRec94: [F32;2],
	fConst95: F32,
	fRec95: [F32;2],
	fConst96: F32,
	fRec96: [F32;2],
	fConst97: F32,
	fRec97: [F32;2],
	fConst98: F32,
	fRec98: [F32;2],
	fConst99: F32,
	fRec99: [F32;2],
}

pub type FaustFloat = F32;
mod ffi {
	use std::os::raw::c_float;
	// Conditionally compile the link attribute only on non-Windows platforms
	#[cfg_attr(not(target_os = "windows"), link(name = "m"))]
	unsafe extern "C" {
		pub fn remainderf(from: c_float, to: c_float) -> c_float;
		pub fn rintf(val: c_float) -> c_float;
	}
}
fn remainder_f32(from: f32, to: f32) -> f32 {
	unsafe { ffi::remainderf(from, to) }
}
fn rint_f32(val: f32) -> f32 {
	unsafe { ffi::rintf(val) }
}

pub const FAUST_INPUTS: usize = 0;
pub const FAUST_OUTPUTS: usize = 1;
pub const FAUST_ACTIVES: usize = 0;
pub const FAUST_PASSIVES: usize = 0;


impl mydsp {
		
	pub fn new() -> mydsp { 
		mydsp {
			iVec0: [0;2],
			fRec0: [0.0;2],
			fSampleRate: 0,
			fConst0: 0.0,
			fConst1: 0.0,
			fRec1: [0.0;2],
			fConst2: 0.0,
			fRec2: [0.0;2],
			fConst3: 0.0,
			fRec3: [0.0;2],
			fConst4: 0.0,
			fRec4: [0.0;2],
			fConst5: 0.0,
			fRec5: [0.0;2],
			fConst6: 0.0,
			fRec6: [0.0;2],
			fConst7: 0.0,
			fRec7: [0.0;2],
			fConst8: 0.0,
			fRec8: [0.0;2],
			fConst9: 0.0,
			fRec9: [0.0;2],
			fConst10: 0.0,
			fRec10: [0.0;2],
			fConst11: 0.0,
			fRec11: [0.0;2],
			fConst12: 0.0,
			fRec12: [0.0;2],
			fConst13: 0.0,
			fRec13: [0.0;2],
			fConst14: 0.0,
			fRec14: [0.0;2],
			fConst15: 0.0,
			fRec15: [0.0;2],
			fConst16: 0.0,
			fRec16: [0.0;2],
			fConst17: 0.0,
			fRec17: [0.0;2],
			fConst18: 0.0,
			fRec18: [0.0;2],
			fConst19: 0.0,
			fRec19: [0.0;2],
			fConst20: 0.0,
			fRec20: [0.0;2],
			fConst21: 0.0,
			fRec21: [0.0;2],
			fConst22: 0.0,
			fRec22: [0.0;2],
			fConst23: 0.0,
			fRec23: [0.0;2],
			fConst24: 0.0,
			fRec24: [0.0;2],
			fConst25: 0.0,
			fRec25: [0.0;2],
			fConst26: 0.0,
			fRec26: [0.0;2],
			fConst27: 0.0,
			fRec27: [0.0;2],
			fConst28: 0.0,
			fRec28: [0.0;2],
			fConst29: 0.0,
			fRec29: [0.0;2],
			fConst30: 0.0,
			fRec30: [0.0;2],
			fConst31: 0.0,
			fRec31: [0.0;2],
			fConst32: 0.0,
			fRec32: [0.0;2],
			fConst33: 0.0,
			fRec33: [0.0;2],
			fConst34: 0.0,
			fRec34: [0.0;2],
			fConst35: 0.0,
			fRec35: [0.0;2],
			fConst36: 0.0,
			fRec36: [0.0;2],
			fConst37: 0.0,
			fRec37: [0.0;2],
			fConst38: 0.0,
			fRec38: [0.0;2],
			fConst39: 0.0,
			fRec39: [0.0;2],
			fConst40: 0.0,
			fRec40: [0.0;2],
			fConst41: 0.0,
			fRec41: [0.0;2],
			fConst42: 0.0,
			fRec42: [0.0;2],
			fConst43: 0.0,
			fRec43: [0.0;2],
			fConst44: 0.0,
			fRec44: [0.0;2],
			fConst45: 0.0,
			fRec45: [0.0;2],
			fConst46: 0.0,
			fRec46: [0.0;2],
			fConst47: 0.0,
			fRec47: [0.0;2],
			fConst48: 0.0,
			fRec48: [0.0;2],
			fConst49: 0.0,
			fRec49: [0.0;2],
			fConst50: 0.0,
			fRec50: [0.0;2],
			fConst51: 0.0,
			fRec51: [0.0;2],
			fConst52: 0.0,
			fRec52: [0.0;2],
			fConst53: 0.0,
			fRec53: [0.0;2],
			fConst54: 0.0,
			fRec54: [0.0;2],
			fConst55: 0.0,
			fRec55: [0.0;2],
			fConst56: 0.0,
			fRec56: [0.0;2],
			fConst57: 0.0,
			fRec57: [0.0;2],
			fConst58: 0.0,
			fRec58: [0.0;2],
			fConst59: 0.0,
			fRec59: [0.0;2],
			fConst60: 0.0,
			fRec60: [0.0;2],
			fConst61: 0.0,
			fRec61: [0.0;2],
			fConst62: 0.0,
			fRec62: [0.0;2],
			fConst63: 0.0,
			fRec63: [0.0;2],
			fConst64: 0.0,
			fRec64: [0.0;2],
			fConst65: 0.0,
			fRec65: [0.0;2],
			fConst66: 0.0,
			fRec66: [0.0;2],
			fConst67: 0.0,
			fRec67: [0.0;2],
			fConst68: 0.0,
			fRec68: [0.0;2],
			fConst69: 0.0,
			fRec69: [0.0;2],
			fConst70: 0.0,
			fRec70: [0.0;2],
			fConst71: 0.0,
			fRec71: [0.0;2],
			fConst72: 0.0,
			fRec72: [0.0;2],
			fConst73: 0.0,
			fRec73: [0.0;2],
			fConst74: 0.0,
			fRec74: [0.0;2],
			fConst75: 0.0,
			fRec75: [0.0;2],
			fConst76: 0.0,
			fRec76: [0.0;2],
			fConst77: 0.0,
			fRec77: [0.0;2],
			fConst78: 0.0,
			fRec78: [0.0;2],
			fConst79: 0.0,
			fRec79: [0.0;2],
			fConst80: 0.0,
			fRec80: [0.0;2],
			fConst81: 0.0,
			fRec81: [0.0;2],
			fConst82: 0.0,
			fRec82: [0.0;2],
			fConst83: 0.0,
			fRec83: [0.0;2],
			fConst84: 0.0,
			fRec84: [0.0;2],
			fConst85: 0.0,
			fRec85: [0.0;2],
			fConst86: 0.0,
			fRec86: [0.0;2],
			fConst87: 0.0,
			fRec87: [0.0;2],
			fConst88: 0.0,
			fRec88: [0.0;2],
			fConst89: 0.0,
			fRec89: [0.0;2],
			fConst90: 0.0,
			fRec90: [0.0;2],
			fConst91: 0.0,
			fRec91: [0.0;2],
			fConst92: 0.0,
			fRec92: [0.0;2],
			fConst93: 0.0,
			fRec93: [0.0;2],
			fConst94: 0.0,
			fRec94: [0.0;2],
			fConst95: 0.0,
			fRec95: [0.0;2],
			fConst96: 0.0,
			fRec96: [0.0;2],
			fConst97: 0.0,
			fRec97: [0.0;2],
			fConst98: 0.0,
			fRec98: [0.0;2],
			fConst99: 0.0,
			fRec99: [0.0;2],
		}
	}
	pub fn metadata(&self, m: &mut dyn Meta) { 
		m.declare("compile_options", r"-lang rust -fpga-mem-th 4 -ct 1 -es 1 -mcd 16 -mdd 1024 -mdy 33 -single -ftz 0");
		m.declare("filename", r"replicate100.dsp");
		m.declare("maths.lib/author", r"GRAME");
		m.declare("maths.lib/copyright", r"GRAME");
		m.declare("maths.lib/license", r"LGPL with exception");
		m.declare("maths.lib/name", r"Faust Math Library");
		m.declare("maths.lib/version", r"2.9.0");
		m.declare("name", r"replicate100");
		m.declare("oscillators.lib/lf_sawpos:author", r"Bart Brouns, revised by Stéphane Letz");
		m.declare("oscillators.lib/lf_sawpos:licence", r"STK-4.3");
		m.declare("oscillators.lib/name", r"Faust Oscillator Library");
		m.declare("oscillators.lib/version", r"1.6.0");
		m.declare("platform.lib/name", r"Generic Platform Library");
		m.declare("platform.lib/version", r"1.3.0");
	}

	pub fn get_sample_rate(&self) -> i32 { self.fSampleRate as i32}
	
	pub fn class_init(sample_rate: i32) {
		// Obtaining locks on 0 static var(s)
	}
	pub fn instance_reset_params(&mut self) {
	}
	pub fn instance_clear(&mut self) {
		for l0 in 0..2 {
			self.iVec0[l0 as usize] = 0;
		}
		for l1 in 0..2 {
			self.fRec0[l1 as usize] = 0.0;
		}
		for l2 in 0..2 {
			self.fRec1[l2 as usize] = 0.0;
		}
		for l3 in 0..2 {
			self.fRec2[l3 as usize] = 0.0;
		}
		for l4 in 0..2 {
			self.fRec3[l4 as usize] = 0.0;
		}
		for l5 in 0..2 {
			self.fRec4[l5 as usize] = 0.0;
		}
		for l6 in 0..2 {
			self.fRec5[l6 as usize] = 0.0;
		}
		for l7 in 0..2 {
			self.fRec6[l7 as usize] = 0.0;
		}
		for l8 in 0..2 {
			self.fRec7[l8 as usize] = 0.0;
		}
		for l9 in 0..2 {
			self.fRec8[l9 as usize] = 0.0;
		}
		for l10 in 0..2 {
			self.fRec9[l10 as usize] = 0.0;
		}
		for l11 in 0..2 {
			self.fRec10[l11 as usize] = 0.0;
		}
		for l12 in 0..2 {
			self.fRec11[l12 as usize] = 0.0;
		}
		for l13 in 0..2 {
			self.fRec12[l13 as usize] = 0.0;
		}
		for l14 in 0..2 {
			self.fRec13[l14 as usize] = 0.0;
		}
		for l15 in 0..2 {
			self.fRec14[l15 as usize] = 0.0;
		}
		for l16 in 0..2 {
			self.fRec15[l16 as usize] = 0.0;
		}
		for l17 in 0..2 {
			self.fRec16[l17 as usize] = 0.0;
		}
		for l18 in 0..2 {
			self.fRec17[l18 as usize] = 0.0;
		}
		for l19 in 0..2 {
			self.fRec18[l19 as usize] = 0.0;
		}
		for l20 in 0..2 {
			self.fRec19[l20 as usize] = 0.0;
		}
		for l21 in 0..2 {
			self.fRec20[l21 as usize] = 0.0;
		}
		for l22 in 0..2 {
			self.fRec21[l22 as usize] = 0.0;
		}
		for l23 in 0..2 {
			self.fRec22[l23 as usize] = 0.0;
		}
		for l24 in 0..2 {
			self.fRec23[l24 as usize] = 0.0;
		}
		for l25 in 0..2 {
			self.fRec24[l25 as usize] = 0.0;
		}
		for l26 in 0..2 {
			self.fRec25[l26 as usize] = 0.0;
		}
		for l27 in 0..2 {
			self.fRec26[l27 as usize] = 0.0;
		}
		for l28 in 0..2 {
			self.fRec27[l28 as usize] = 0.0;
		}
		for l29 in 0..2 {
			self.fRec28[l29 as usize] = 0.0;
		}
		for l30 in 0..2 {
			self.fRec29[l30 as usize] = 0.0;
		}
		for l31 in 0..2 {
			self.fRec30[l31 as usize] = 0.0;
		}
		for l32 in 0..2 {
			self.fRec31[l32 as usize] = 0.0;
		}
		for l33 in 0..2 {
			self.fRec32[l33 as usize] = 0.0;
		}
		for l34 in 0..2 {
			self.fRec33[l34 as usize] = 0.0;
		}
		for l35 in 0..2 {
			self.fRec34[l35 as usize] = 0.0;
		}
		for l36 in 0..2 {
			self.fRec35[l36 as usize] = 0.0;
		}
		for l37 in 0..2 {
			self.fRec36[l37 as usize] = 0.0;
		}
		for l38 in 0..2 {
			self.fRec37[l38 as usize] = 0.0;
		}
		for l39 in 0..2 {
			self.fRec38[l39 as usize] = 0.0;
		}
		for l40 in 0..2 {
			self.fRec39[l40 as usize] = 0.0;
		}
		for l41 in 0..2 {
			self.fRec40[l41 as usize] = 0.0;
		}
		for l42 in 0..2 {
			self.fRec41[l42 as usize] = 0.0;
		}
		for l43 in 0..2 {
			self.fRec42[l43 as usize] = 0.0;
		}
		for l44 in 0..2 {
			self.fRec43[l44 as usize] = 0.0;
		}
		for l45 in 0..2 {
			self.fRec44[l45 as usize] = 0.0;
		}
		for l46 in 0..2 {
			self.fRec45[l46 as usize] = 0.0;
		}
		for l47 in 0..2 {
			self.fRec46[l47 as usize] = 0.0;
		}
		for l48 in 0..2 {
			self.fRec47[l48 as usize] = 0.0;
		}
		for l49 in 0..2 {
			self.fRec48[l49 as usize] = 0.0;
		}
		for l50 in 0..2 {
			self.fRec49[l50 as usize] = 0.0;
		}
		for l51 in 0..2 {
			self.fRec50[l51 as usize] = 0.0;
		}
		for l52 in 0..2 {
			self.fRec51[l52 as usize] = 0.0;
		}
		for l53 in 0..2 {
			self.fRec52[l53 as usize] = 0.0;
		}
		for l54 in 0..2 {
			self.fRec53[l54 as usize] = 0.0;
		}
		for l55 in 0..2 {
			self.fRec54[l55 as usize] = 0.0;
		}
		for l56 in 0..2 {
			self.fRec55[l56 as usize] = 0.0;
		}
		for l57 in 0..2 {
			self.fRec56[l57 as usize] = 0.0;
		}
		for l58 in 0..2 {
			self.fRec57[l58 as usize] = 0.0;
		}
		for l59 in 0..2 {
			self.fRec58[l59 as usize] = 0.0;
		}
		for l60 in 0..2 {
			self.fRec59[l60 as usize] = 0.0;
		}
		for l61 in 0..2 {
			self.fRec60[l61 as usize] = 0.0;
		}
		for l62 in 0..2 {
			self.fRec61[l62 as usize] = 0.0;
		}
		for l63 in 0..2 {
			self.fRec62[l63 as usize] = 0.0;
		}
		for l64 in 0..2 {
			self.fRec63[l64 as usize] = 0.0;
		}
		for l65 in 0..2 {
			self.fRec64[l65 as usize] = 0.0;
		}
		for l66 in 0..2 {
			self.fRec65[l66 as usize] = 0.0;
		}
		for l67 in 0..2 {
			self.fRec66[l67 as usize] = 0.0;
		}
		for l68 in 0..2 {
			self.fRec67[l68 as usize] = 0.0;
		}
		for l69 in 0..2 {
			self.fRec68[l69 as usize] = 0.0;
		}
		for l70 in 0..2 {
			self.fRec69[l70 as usize] = 0.0;
		}
		for l71 in 0..2 {
			self.fRec70[l71 as usize] = 0.0;
		}
		for l72 in 0..2 {
			self.fRec71[l72 as usize] = 0.0;
		}
		for l73 in 0..2 {
			self.fRec72[l73 as usize] = 0.0;
		}
		for l74 in 0..2 {
			self.fRec73[l74 as usize] = 0.0;
		}
		for l75 in 0..2 {
			self.fRec74[l75 as usize] = 0.0;
		}
		for l76 in 0..2 {
			self.fRec75[l76 as usize] = 0.0;
		}
		for l77 in 0..2 {
			self.fRec76[l77 as usize] = 0.0;
		}
		for l78 in 0..2 {
			self.fRec77[l78 as usize] = 0.0;
		}
		for l79 in 0..2 {
			self.fRec78[l79 as usize] = 0.0;
		}
		for l80 in 0..2 {
			self.fRec79[l80 as usize] = 0.0;
		}
		for l81 in 0..2 {
			self.fRec80[l81 as usize] = 0.0;
		}
		for l82 in 0..2 {
			self.fRec81[l82 as usize] = 0.0;
		}
		for l83 in 0..2 {
			self.fRec82[l83 as usize] = 0.0;
		}
		for l84 in 0..2 {
			self.fRec83[l84 as usize] = 0.0;
		}
		for l85 in 0..2 {
			self.fRec84[l85 as usize] = 0.0;
		}
		for l86 in 0..2 {
			self.fRec85[l86 as usize] = 0.0;
		}
		for l87 in 0..2 {
			self.fRec86[l87 as usize] = 0.0;
		}
		for l88 in 0..2 {
			self.fRec87[l88 as usize] = 0.0;
		}
		for l89 in 0..2 {
			self.fRec88[l89 as usize] = 0.0;
		}
		for l90 in 0..2 {
			self.fRec89[l90 as usize] = 0.0;
		}
		for l91 in 0..2 {
			self.fRec90[l91 as usize] = 0.0;
		}
		for l92 in 0..2 {
			self.fRec91[l92 as usize] = 0.0;
		}
		for l93 in 0..2 {
			self.fRec92[l93 as usize] = 0.0;
		}
		for l94 in 0..2 {
			self.fRec93[l94 as usize] = 0.0;
		}
		for l95 in 0..2 {
			self.fRec94[l95 as usize] = 0.0;
		}
		for l96 in 0..2 {
			self.fRec95[l96 as usize] = 0.0;
		}
		for l97 in 0..2 {
			self.fRec96[l97 as usize] = 0.0;
		}
		for l98 in 0..2 {
			self.fRec97[l98 as usize] = 0.0;
		}
		for l99 in 0..2 {
			self.fRec98[l99 as usize] = 0.0;
		}
		for l100 in 0..2 {
			self.fRec99[l100 as usize] = 0.0;
		}
	}
	pub fn instance_constants(&mut self, sample_rate: i32) {
		// Obtaining locks on 0 static var(s)
		self.fSampleRate = sample_rate;
		self.fConst0 = F32::min(1.92e+05, F32::max(1.0, (self.fSampleRate) as F32));
		self.fConst1 = 5e+01 / self.fConst0;
		self.fConst2 = 1e+02 / self.fConst0;
		self.fConst3 = 1.5e+02 / self.fConst0;
		self.fConst4 = 2e+02 / self.fConst0;
		self.fConst5 = 2.5e+02 / self.fConst0;
		self.fConst6 = 3e+02 / self.fConst0;
		self.fConst7 = 3.5e+02 / self.fConst0;
		self.fConst8 = 4e+02 / self.fConst0;
		self.fConst9 = 4.5e+02 / self.fConst0;
		self.fConst10 = 5e+02 / self.fConst0;
		self.fConst11 = 5.5e+02 / self.fConst0;
		self.fConst12 = 6e+02 / self.fConst0;
		self.fConst13 = 6.5e+02 / self.fConst0;
		self.fConst14 = 7e+02 / self.fConst0;
		self.fConst15 = 7.5e+02 / self.fConst0;
		self.fConst16 = 8e+02 / self.fConst0;
		self.fConst17 = 8.5e+02 / self.fConst0;
		self.fConst18 = 9e+02 / self.fConst0;
		self.fConst19 = 9.5e+02 / self.fConst0;
		self.fConst20 = 1e+03 / self.fConst0;
		self.fConst21 = 1.05e+03 / self.fConst0;
		self.fConst22 = 1.1e+03 / self.fConst0;
		self.fConst23 = 1.15e+03 / self.fConst0;
		self.fConst24 = 1.2e+03 / self.fConst0;
		self.fConst25 = 1.25e+03 / self.fConst0;
		self.fConst26 = 1.3e+03 / self.fConst0;
		self.fConst27 = 1.35e+03 / self.fConst0;
		self.fConst28 = 1.4e+03 / self.fConst0;
		self.fConst29 = 1.45e+03 / self.fConst0;
		self.fConst30 = 1.5e+03 / self.fConst0;
		self.fConst31 = 1.55e+03 / self.fConst0;
		self.fConst32 = 1.6e+03 / self.fConst0;
		self.fConst33 = 1.65e+03 / self.fConst0;
		self.fConst34 = 1.7e+03 / self.fConst0;
		self.fConst35 = 1.75e+03 / self.fConst0;
		self.fConst36 = 1.8e+03 / self.fConst0;
		self.fConst37 = 1.85e+03 / self.fConst0;
		self.fConst38 = 1.9e+03 / self.fConst0;
		self.fConst39 = 1.95e+03 / self.fConst0;
		self.fConst40 = 2e+03 / self.fConst0;
		self.fConst41 = 2.05e+03 / self.fConst0;
		self.fConst42 = 2.1e+03 / self.fConst0;
		self.fConst43 = 2.15e+03 / self.fConst0;
		self.fConst44 = 2.2e+03 / self.fConst0;
		self.fConst45 = 2.25e+03 / self.fConst0;
		self.fConst46 = 2.3e+03 / self.fConst0;
		self.fConst47 = 2.35e+03 / self.fConst0;
		self.fConst48 = 2.4e+03 / self.fConst0;
		self.fConst49 = 2.45e+03 / self.fConst0;
		self.fConst50 = 2.5e+03 / self.fConst0;
		self.fConst51 = 2.55e+03 / self.fConst0;
		self.fConst52 = 2.6e+03 / self.fConst0;
		self.fConst53 = 2.65e+03 / self.fConst0;
		self.fConst54 = 2.7e+03 / self.fConst0;
		self.fConst55 = 2.75e+03 / self.fConst0;
		self.fConst56 = 2.8e+03 / self.fConst0;
		self.fConst57 = 2.85e+03 / self.fConst0;
		self.fConst58 = 2.9e+03 / self.fConst0;
		self.fConst59 = 2.95e+03 / self.fConst0;
		self.fConst60 = 3e+03 / self.fConst0;
		self.fConst61 = 3.05e+03 / self.fConst0;
		self.fConst62 = 3.1e+03 / self.fConst0;
		self.fConst63 = 3.15e+03 / self.fConst0;
		self.fConst64 = 3.2e+03 / self.fConst0;
		self.fConst65 = 3.25e+03 / self.fConst0;
		self.fConst66 = 3.3e+03 / self.fConst0;
		self.fConst67 = 3.35e+03 / self.fConst0;
		self.fConst68 = 3.4e+03 / self.fConst0;
		self.fConst69 = 3.45e+03 / self.fConst0;
		self.fConst70 = 3.5e+03 / self.fConst0;
		self.fConst71 = 3.55e+03 / self.fConst0;
		self.fConst72 = 3.6e+03 / self.fConst0;
		self.fConst73 = 3.65e+03 / self.fConst0;
		self.fConst74 = 3.7e+03 / self.fConst0;
		self.fConst75 = 3.75e+03 / self.fConst0;
		self.fConst76 = 3.8e+03 / self.fConst0;
		self.fConst77 = 3.85e+03 / self.fConst0;
		self.fConst78 = 3.9e+03 / self.fConst0;
		self.fConst79 = 3.95e+03 / self.fConst0;
		self.fConst80 = 4e+03 / self.fConst0;
		self.fConst81 = 4.05e+03 / self.fConst0;
		self.fConst82 = 4.1e+03 / self.fConst0;
		self.fConst83 = 4.15e+03 / self.fConst0;
		self.fConst84 = 4.2e+03 / self.fConst0;
		self.fConst85 = 4.25e+03 / self.fConst0;
		self.fConst86 = 4.3e+03 / self.fConst0;
		self.fConst87 = 4.35e+03 / self.fConst0;
		self.fConst88 = 4.4e+03 / self.fConst0;
		self.fConst89 = 4.45e+03 / self.fConst0;
		self.fConst90 = 4.5e+03 / self.fConst0;
		self.fConst91 = 4.55e+03 / self.fConst0;
		self.fConst92 = 4.6e+03 / self.fConst0;
		self.fConst93 = 4.65e+03 / self.fConst0;
		self.fConst94 = 4.7e+03 / self.fConst0;
		self.fConst95 = 4.75e+03 / self.fConst0;
		self.fConst96 = 4.8e+03 / self.fConst0;
		self.fConst97 = 4.85e+03 / self.fConst0;
		self.fConst98 = 4.9e+03 / self.fConst0;
		self.fConst99 = 4.95e+03 / self.fConst0;
	}
	pub fn instance_init(&mut self, sample_rate: i32) {
		self.instance_constants(sample_rate);
		self.instance_reset_params();
		self.instance_clear();
	}
	pub fn init(&mut self, sample_rate: i32) {
		mydsp::class_init(sample_rate);
		self.instance_init(sample_rate);
	}
	
	pub fn build_user_interface(&self, ui_interface: &mut dyn UI<FaustFloat>) {
		Self::build_user_interface_static(ui_interface);
	}
	
	pub fn build_user_interface_static(ui_interface: &mut dyn UI<FaustFloat>) {
		ui_interface.open_vertical_box("replicate100");
		ui_interface.close_box();
	}
	
	pub fn get_param(&self, param: ParamIndex) -> Option<FaustFloat> {
		match param.0 {
			_ => None,
		}
	}
	
	pub fn set_param(&mut self, param: ParamIndex, value: FaustFloat) {
		match param.0 {
			_ => {}
		}
	}
	
	pub fn compute(
		&mut self,
		count: usize,
		inputs: &[impl AsRef<[FaustFloat]>],
		outputs: &mut[impl AsMut<[FaustFloat]>],
	) {
		
		// Obtaining locks on 0 static var(s)
		let [outputs0, .. ] = outputs.as_mut() else { panic!("wrong number of output buffers"); };
		let outputs0 = outputs0.as_mut()[..count].iter_mut();
		let zipped_iterators = outputs0;
		for output0 in zipped_iterators {
			self.iVec0[0] = 1;
			let mut iTemp0: i32 = i32::wrapping_sub(1, self.iVec0[1]);
			let mut fTemp1: F32 = (if iTemp0 != 0 {0.0} else {self.fRec0[1]});
			self.fRec0[0] = fTemp1 - F32::floor(fTemp1);
			let mut fTemp2: F32 = (if iTemp0 != 0 {0.0} else {self.fConst1 + self.fRec1[1]});
			self.fRec1[0] = fTemp2 - F32::floor(fTemp2);
			let mut fTemp3: F32 = (if iTemp0 != 0 {0.0} else {self.fConst2 + self.fRec2[1]});
			self.fRec2[0] = fTemp3 - F32::floor(fTemp3);
			let mut fTemp4: F32 = (if iTemp0 != 0 {0.0} else {self.fConst3 + self.fRec3[1]});
			self.fRec3[0] = fTemp4 - F32::floor(fTemp4);
			let mut fTemp5: F32 = (if iTemp0 != 0 {0.0} else {self.fConst4 + self.fRec4[1]});
			self.fRec4[0] = fTemp5 - F32::floor(fTemp5);
			let mut fTemp6: F32 = (if iTemp0 != 0 {0.0} else {self.fConst5 + self.fRec5[1]});
			self.fRec5[0] = fTemp6 - F32::floor(fTemp6);
			let mut fTemp7: F32 = (if iTemp0 != 0 {0.0} else {self.fConst6 + self.fRec6[1]});
			self.fRec6[0] = fTemp7 - F32::floor(fTemp7);
			let mut fTemp8: F32 = (if iTemp0 != 0 {0.0} else {self.fConst7 + self.fRec7[1]});
			self.fRec7[0] = fTemp8 - F32::floor(fTemp8);
			let mut fTemp9: F32 = (if iTemp0 != 0 {0.0} else {self.fConst8 + self.fRec8[1]});
			self.fRec8[0] = fTemp9 - F32::floor(fTemp9);
			let mut fTemp10: F32 = (if iTemp0 != 0 {0.0} else {self.fConst9 + self.fRec9[1]});
			self.fRec9[0] = fTemp10 - F32::floor(fTemp10);
			let mut fTemp11: F32 = (if iTemp0 != 0 {0.0} else {self.fConst10 + self.fRec10[1]});
			self.fRec10[0] = fTemp11 - F32::floor(fTemp11);
			let mut fTemp12: F32 = (if iTemp0 != 0 {0.0} else {self.fConst11 + self.fRec11[1]});
			self.fRec11[0] = fTemp12 - F32::floor(fTemp12);
			let mut fTemp13: F32 = (if iTemp0 != 0 {0.0} else {self.fConst12 + self.fRec12[1]});
			self.fRec12[0] = fTemp13 - F32::floor(fTemp13);
			let mut fTemp14: F32 = (if iTemp0 != 0 {0.0} else {self.fConst13 + self.fRec13[1]});
			self.fRec13[0] = fTemp14 - F32::floor(fTemp14);
			let mut fTemp15: F32 = (if iTemp0 != 0 {0.0} else {self.fConst14 + self.fRec14[1]});
			self.fRec14[0] = fTemp15 - F32::floor(fTemp15);
			let mut fTemp16: F32 = (if iTemp0 != 0 {0.0} else {self.fConst15 + self.fRec15[1]});
			self.fRec15[0] = fTemp16 - F32::floor(fTemp16);
			let mut fTemp17: F32 = (if iTemp0 != 0 {0.0} else {self.fConst16 + self.fRec16[1]});
			self.fRec16[0] = fTemp17 - F32::floor(fTemp17);
			let mut fTemp18: F32 = (if iTemp0 != 0 {0.0} else {self.fConst17 + self.fRec17[1]});
			self.fRec17[0] = fTemp18 - F32::floor(fTemp18);
			let mut fTemp19: F32 = (if iTemp0 != 0 {0.0} else {self.fConst18 + self.fRec18[1]});
			self.fRec18[0] = fTemp19 - F32::floor(fTemp19);
			let mut fTemp20: F32 = (if iTemp0 != 0 {0.0} else {self.fConst19 + self.fRec19[1]});
			self.fRec19[0] = fTemp20 - F32::floor(fTemp20);
			let mut fTemp21: F32 = (if iTemp0 != 0 {0.0} else {self.fConst20 + self.fRec20[1]});
			self.fRec20[0] = fTemp21 - F32::floor(fTemp21);
			let mut fTemp22: F32 = (if iTemp0 != 0 {0.0} else {self.fConst21 + self.fRec21[1]});
			self.fRec21[0] = fTemp22 - F32::floor(fTemp22);
			let mut fTemp23: F32 = (if iTemp0 != 0 {0.0} else {self.fConst22 + self.fRec22[1]});
			self.fRec22[0] = fTemp23 - F32::floor(fTemp23);
			let mut fTemp24: F32 = (if iTemp0 != 0 {0.0} else {self.fConst23 + self.fRec23[1]});
			self.fRec23[0] = fTemp24 - F32::floor(fTemp24);
			let mut fTemp25: F32 = (if iTemp0 != 0 {0.0} else {self.fConst24 + self.fRec24[1]});
			self.fRec24[0] = fTemp25 - F32::floor(fTemp25);
			let mut fTemp26: F32 = (if iTemp0 != 0 {0.0} else {self.fConst25 + self.fRec25[1]});
			self.fRec25[0] = fTemp26 - F32::floor(fTemp26);
			let mut fTemp27: F32 = (if iTemp0 != 0 {0.0} else {self.fConst26 + self.fRec26[1]});
			self.fRec26[0] = fTemp27 - F32::floor(fTemp27);
			let mut fTemp28: F32 = (if iTemp0 != 0 {0.0} else {self.fConst27 + self.fRec27[1]});
			self.fRec27[0] = fTemp28 - F32::floor(fTemp28);
			let mut fTemp29: F32 = (if iTemp0 != 0 {0.0} else {self.fConst28 + self.fRec28[1]});
			self.fRec28[0] = fTemp29 - F32::floor(fTemp29);
			let mut fTemp30: F32 = (if iTemp0 != 0 {0.0} else {self.fConst29 + self.fRec29[1]});
			self.fRec29[0] = fTemp30 - F32::floor(fTemp30);
			let mut fTemp31: F32 = (if iTemp0 != 0 {0.0} else {self.fConst30 + self.fRec30[1]});
			self.fRec30[0] = fTemp31 - F32::floor(fTemp31);
			let mut fTemp32: F32 = (if iTemp0 != 0 {0.0} else {self.fConst31 + self.fRec31[1]});
			self.fRec31[0] = fTemp32 - F32::floor(fTemp32);
			let mut fTemp33: F32 = (if iTemp0 != 0 {0.0} else {self.fConst32 + self.fRec32[1]});
			self.fRec32[0] = fTemp33 - F32::floor(fTemp33);
			let mut fTemp34: F32 = (if iTemp0 != 0 {0.0} else {self.fConst33 + self.fRec33[1]});
			self.fRec33[0] = fTemp34 - F32::floor(fTemp34);
			let mut fTemp35: F32 = (if iTemp0 != 0 {0.0} else {self.fConst34 + self.fRec34[1]});
			self.fRec34[0] = fTemp35 - F32::floor(fTemp35);
			let mut fTemp36: F32 = (if iTemp0 != 0 {0.0} else {self.fConst35 + self.fRec35[1]});
			self.fRec35[0] = fTemp36 - F32::floor(fTemp36);
			let mut fTemp37: F32 = (if iTemp0 != 0 {0.0} else {self.fConst36 + self.fRec36[1]});
			self.fRec36[0] = fTemp37 - F32::floor(fTemp37);
			let mut fTemp38: F32 = (if iTemp0 != 0 {0.0} else {self.fConst37 + self.fRec37[1]});
			self.fRec37[0] = fTemp38 - F32::floor(fTemp38);
			let mut fTemp39: F32 = (if iTemp0 != 0 {0.0} else {self.fConst38 + self.fRec38[1]});
			self.fRec38[0] = fTemp39 - F32::floor(fTemp39);
			let mut fTemp40: F32 = (if iTemp0 != 0 {0.0} else {self.fConst39 + self.fRec39[1]});
			self.fRec39[0] = fTemp40 - F32::floor(fTemp40);
			let mut fTemp41: F32 = (if iTemp0 != 0 {0.0} else {self.fConst40 + self.fRec40[1]});
			self.fRec40[0] = fTemp41 - F32::floor(fTemp41);
			let mut fTemp42: F32 = (if iTemp0 != 0 {0.0} else {self.fConst41 + self.fRec41[1]});
			self.fRec41[0] = fTemp42 - F32::floor(fTemp42);
			let mut fTemp43: F32 = (if iTemp0 != 0 {0.0} else {self.fConst42 + self.fRec42[1]});
			self.fRec42[0] = fTemp43 - F32::floor(fTemp43);
			let mut fTemp44: F32 = (if iTemp0 != 0 {0.0} else {self.fConst43 + self.fRec43[1]});
			self.fRec43[0] = fTemp44 - F32::floor(fTemp44);
			let mut fTemp45: F32 = (if iTemp0 != 0 {0.0} else {self.fConst44 + self.fRec44[1]});
			self.fRec44[0] = fTemp45 - F32::floor(fTemp45);
			let mut fTemp46: F32 = (if iTemp0 != 0 {0.0} else {self.fConst45 + self.fRec45[1]});
			self.fRec45[0] = fTemp46 - F32::floor(fTemp46);
			let mut fTemp47: F32 = (if iTemp0 != 0 {0.0} else {self.fConst46 + self.fRec46[1]});
			self.fRec46[0] = fTemp47 - F32::floor(fTemp47);
			let mut fTemp48: F32 = (if iTemp0 != 0 {0.0} else {self.fConst47 + self.fRec47[1]});
			self.fRec47[0] = fTemp48 - F32::floor(fTemp48);
			let mut fTemp49: F32 = (if iTemp0 != 0 {0.0} else {self.fConst48 + self.fRec48[1]});
			self.fRec48[0] = fTemp49 - F32::floor(fTemp49);
			let mut fTemp50: F32 = (if iTemp0 != 0 {0.0} else {self.fConst49 + self.fRec49[1]});
			self.fRec49[0] = fTemp50 - F32::floor(fTemp50);
			let mut fTemp51: F32 = (if iTemp0 != 0 {0.0} else {self.fConst50 + self.fRec50[1]});
			self.fRec50[0] = fTemp51 - F32::floor(fTemp51);
			let mut fTemp52: F32 = (if iTemp0 != 0 {0.0} else {self.fConst51 + self.fRec51[1]});
			self.fRec51[0] = fTemp52 - F32::floor(fTemp52);
			let mut fTemp53: F32 = (if iTemp0 != 0 {0.0} else {self.fConst52 + self.fRec52[1]});
			self.fRec52[0] = fTemp53 - F32::floor(fTemp53);
			let mut fTemp54: F32 = (if iTemp0 != 0 {0.0} else {self.fConst53 + self.fRec53[1]});
			self.fRec53[0] = fTemp54 - F32::floor(fTemp54);
			let mut fTemp55: F32 = (if iTemp0 != 0 {0.0} else {self.fConst54 + self.fRec54[1]});
			self.fRec54[0] = fTemp55 - F32::floor(fTemp55);
			let mut fTemp56: F32 = (if iTemp0 != 0 {0.0} else {self.fConst55 + self.fRec55[1]});
			self.fRec55[0] = fTemp56 - F32::floor(fTemp56);
			let mut fTemp57: F32 = (if iTemp0 != 0 {0.0} else {self.fConst56 + self.fRec56[1]});
			self.fRec56[0] = fTemp57 - F32::floor(fTemp57);
			let mut fTemp58: F32 = (if iTemp0 != 0 {0.0} else {self.fConst57 + self.fRec57[1]});
			self.fRec57[0] = fTemp58 - F32::floor(fTemp58);
			let mut fTemp59: F32 = (if iTemp0 != 0 {0.0} else {self.fConst58 + self.fRec58[1]});
			self.fRec58[0] = fTemp59 - F32::floor(fTemp59);
			let mut fTemp60: F32 = (if iTemp0 != 0 {0.0} else {self.fConst59 + self.fRec59[1]});
			self.fRec59[0] = fTemp60 - F32::floor(fTemp60);
			let mut fTemp61: F32 = (if iTemp0 != 0 {0.0} else {self.fConst60 + self.fRec60[1]});
			self.fRec60[0] = fTemp61 - F32::floor(fTemp61);
			let mut fTemp62: F32 = (if iTemp0 != 0 {0.0} else {self.fConst61 + self.fRec61[1]});
			self.fRec61[0] = fTemp62 - F32::floor(fTemp62);
			let mut fTemp63: F32 = (if iTemp0 != 0 {0.0} else {self.fConst62 + self.fRec62[1]});
			self.fRec62[0] = fTemp63 - F32::floor(fTemp63);
			let mut fTemp64: F32 = (if iTemp0 != 0 {0.0} else {self.fConst63 + self.fRec63[1]});
			self.fRec63[0] = fTemp64 - F32::floor(fTemp64);
			let mut fTemp65: F32 = (if iTemp0 != 0 {0.0} else {self.fConst64 + self.fRec64[1]});
			self.fRec64[0] = fTemp65 - F32::floor(fTemp65);
			let mut fTemp66: F32 = (if iTemp0 != 0 {0.0} else {self.fConst65 + self.fRec65[1]});
			self.fRec65[0] = fTemp66 - F32::floor(fTemp66);
			let mut fTemp67: F32 = (if iTemp0 != 0 {0.0} else {self.fConst66 + self.fRec66[1]});
			self.fRec66[0] = fTemp67 - F32::floor(fTemp67);
			let mut fTemp68: F32 = (if iTemp0 != 0 {0.0} else {self.fConst67 + self.fRec67[1]});
			self.fRec67[0] = fTemp68 - F32::floor(fTemp68);
			let mut fTemp69: F32 = (if iTemp0 != 0 {0.0} else {self.fConst68 + self.fRec68[1]});
			self.fRec68[0] = fTemp69 - F32::floor(fTemp69);
			let mut fTemp70: F32 = (if iTemp0 != 0 {0.0} else {self.fConst69 + self.fRec69[1]});
			self.fRec69[0] = fTemp70 - F32::floor(fTemp70);
			let mut fTemp71: F32 = (if iTemp0 != 0 {0.0} else {self.fConst70 + self.fRec70[1]});
			self.fRec70[0] = fTemp71 - F32::floor(fTemp71);
			let mut fTemp72: F32 = (if iTemp0 != 0 {0.0} else {self.fConst71 + self.fRec71[1]});
			self.fRec71[0] = fTemp72 - F32::floor(fTemp72);
			let mut fTemp73: F32 = (if iTemp0 != 0 {0.0} else {self.fConst72 + self.fRec72[1]});
			self.fRec72[0] = fTemp73 - F32::floor(fTemp73);
			let mut fTemp74: F32 = (if iTemp0 != 0 {0.0} else {self.fConst73 + self.fRec73[1]});
			self.fRec73[0] = fTemp74 - F32::floor(fTemp74);
			let mut fTemp75: F32 = (if iTemp0 != 0 {0.0} else {self.fConst74 + self.fRec74[1]});
			self.fRec74[0] = fTemp75 - F32::floor(fTemp75);
			let mut fTemp76: F32 = (if iTemp0 != 0 {0.0} else {self.fConst75 + self.fRec75[1]});
			self.fRec75[0] = fTemp76 - F32::floor(fTemp76);
			let mut fTemp77: F32 = (if iTemp0 != 0 {0.0} else {self.fConst76 + self.fRec76[1]});
			self.fRec76[0] = fTemp77 - F32::floor(fTemp77);
			let mut fTemp78: F32 = (if iTemp0 != 0 {0.0} else {self.fConst77 + self.fRec77[1]});
			self.fRec77[0] = fTemp78 - F32::floor(fTemp78);
			let mut fTemp79: F32 = (if iTemp0 != 0 {0.0} else {self.fConst78 + self.fRec78[1]});
			self.fRec78[0] = fTemp79 - F32::floor(fTemp79);
			let mut fTemp80: F32 = (if iTemp0 != 0 {0.0} else {self.fConst79 + self.fRec79[1]});
			self.fRec79[0] = fTemp80 - F32::floor(fTemp80);
			let mut fTemp81: F32 = (if iTemp0 != 0 {0.0} else {self.fConst80 + self.fRec80[1]});
			self.fRec80[0] = fTemp81 - F32::floor(fTemp81);
			let mut fTemp82: F32 = (if iTemp0 != 0 {0.0} else {self.fConst81 + self.fRec81[1]});
			self.fRec81[0] = fTemp82 - F32::floor(fTemp82);
			let mut fTemp83: F32 = (if iTemp0 != 0 {0.0} else {self.fConst82 + self.fRec82[1]});
			self.fRec82[0] = fTemp83 - F32::floor(fTemp83);
			let mut fTemp84: F32 = (if iTemp0 != 0 {0.0} else {self.fConst83 + self.fRec83[1]});
			self.fRec83[0] = fTemp84 - F32::floor(fTemp84);
			let mut fTemp85: F32 = (if iTemp0 != 0 {0.0} else {self.fConst84 + self.fRec84[1]});
			self.fRec84[0] = fTemp85 - F32::floor(fTemp85);
			let mut fTemp86: F32 = (if iTemp0 != 0 {0.0} else {self.fConst85 + self.fRec85[1]});
			self.fRec85[0] = fTemp86 - F32::floor(fTemp86);
			let mut fTemp87: F32 = (if iTemp0 != 0 {0.0} else {self.fConst86 + self.fRec86[1]});
			self.fRec86[0] = fTemp87 - F32::floor(fTemp87);
			let mut fTemp88: F32 = (if iTemp0 != 0 {0.0} else {self.fConst87 + self.fRec87[1]});
			self.fRec87[0] = fTemp88 - F32::floor(fTemp88);
			let mut fTemp89: F32 = (if iTemp0 != 0 {0.0} else {self.fConst88 + self.fRec88[1]});
			self.fRec88[0] = fTemp89 - F32::floor(fTemp89);
			let mut fTemp90: F32 = (if iTemp0 != 0 {0.0} else {self.fConst89 + self.fRec89[1]});
			self.fRec89[0] = fTemp90 - F32::floor(fTemp90);
			let mut fTemp91: F32 = (if iTemp0 != 0 {0.0} else {self.fConst90 + self.fRec90[1]});
			self.fRec90[0] = fTemp91 - F32::floor(fTemp91);
			let mut fTemp92: F32 = (if iTemp0 != 0 {0.0} else {self.fConst91 + self.fRec91[1]});
			self.fRec91[0] = fTemp92 - F32::floor(fTemp92);
			let mut fTemp93: F32 = (if iTemp0 != 0 {0.0} else {self.fConst92 + self.fRec92[1]});
			self.fRec92[0] = fTemp93 - F32::floor(fTemp93);
			let mut fTemp94: F32 = (if iTemp0 != 0 {0.0} else {self.fConst93 + self.fRec93[1]});
			self.fRec93[0] = fTemp94 - F32::floor(fTemp94);
			let mut fTemp95: F32 = (if iTemp0 != 0 {0.0} else {self.fConst94 + self.fRec94[1]});
			self.fRec94[0] = fTemp95 - F32::floor(fTemp95);
			let mut fTemp96: F32 = (if iTemp0 != 0 {0.0} else {self.fConst95 + self.fRec95[1]});
			self.fRec95[0] = fTemp96 - F32::floor(fTemp96);
			let mut fTemp97: F32 = (if iTemp0 != 0 {0.0} else {self.fConst96 + self.fRec96[1]});
			self.fRec96[0] = fTemp97 - F32::floor(fTemp97);
			let mut fTemp98: F32 = (if iTemp0 != 0 {0.0} else {self.fConst97 + self.fRec97[1]});
			self.fRec97[0] = fTemp98 - F32::floor(fTemp98);
			let mut fTemp99: F32 = (if iTemp0 != 0 {0.0} else {self.fConst98 + self.fRec98[1]});
			self.fRec98[0] = fTemp99 - F32::floor(fTemp99);
			let mut fTemp100: F32 = (if iTemp0 != 0 {0.0} else {self.fConst99 + self.fRec99[1]});
			self.fRec99[0] = fTemp100 - F32::floor(fTemp100);
			*output0 = F32::sin(6.2831855 * self.fRec0[0]) + F32::sin(6.2831855 * self.fRec1[0]) + 0.5 * F32::sin(6.2831855 * self.fRec2[0]) + 0.33333334 * F32::sin(6.2831855 * self.fRec3[0]) + 0.25 * F32::sin(6.2831855 * self.fRec4[0]) + 0.2 * F32::sin(6.2831855 * self.fRec5[0]) + 0.16666667 * F32::sin(6.2831855 * self.fRec6[0]) + 0.14285715 * F32::sin(6.2831855 * self.fRec7[0]) + 0.125 * F32::sin(6.2831855 * self.fRec8[0]) + 0.11111111 * F32::sin(6.2831855 * self.fRec9[0]) + 0.1 * F32::sin(6.2831855 * self.fRec10[0]) + 0.09090909 * F32::sin(6.2831855 * self.fRec11[0]) + 0.083333336 * F32::sin(6.2831855 * self.fRec12[0]) + 0.07692308 * F32::sin(6.2831855 * self.fRec13[0]) + 0.071428575 * F32::sin(6.2831855 * self.fRec14[0]) + 0.06666667 * F32::sin(6.2831855 * self.fRec15[0]) + 0.0625 * F32::sin(6.2831855 * self.fRec16[0]) + 0.05882353 * F32::sin(6.2831855 * self.fRec17[0]) + 0.055555556 * F32::sin(6.2831855 * self.fRec18[0]) + 0.05263158 * F32::sin(6.2831855 * self.fRec19[0]) + 0.05 * F32::sin(6.2831855 * self.fRec20[0]) + 0.04761905 * F32::sin(6.2831855 * self.fRec21[0]) + 0.045454547 * F32::sin(6.2831855 * self.fRec22[0]) + 0.04347826 * F32::sin(6.2831855 * self.fRec23[0]) + 0.041666668 * F32::sin(6.2831855 * self.fRec24[0]) + 0.04 * F32::sin(6.2831855 * self.fRec25[0]) + 0.03846154 * F32::sin(6.2831855 * self.fRec26[0]) + 0.037037037 * F32::sin(6.2831855 * self.fRec27[0]) + 0.035714287 * F32::sin(6.2831855 * self.fRec28[0]) + 0.03448276 * F32::sin(6.2831855 * self.fRec29[0]) + 0.033333335 * F32::sin(6.2831855 * self.fRec30[0]) + 0.032258064 * F32::sin(6.2831855 * self.fRec31[0]) + 0.03125 * F32::sin(6.2831855 * self.fRec32[0]) + 0.030303031 * F32::sin(6.2831855 * self.fRec33[0]) + 0.029411765 * F32::sin(6.2831855 * self.fRec34[0]) + 0.028571429 * F32::sin(6.2831855 * self.fRec35[0]) + 0.027777778 * F32::sin(6.2831855 * self.fRec36[0]) + 0.027027028 * F32::sin(6.2831855 * self.fRec37[0]) + 0.02631579 * F32::sin(6.2831855 * self.fRec38[0]) + 0.025641026 * F32::sin(6.2831855 * self.fRec39[0]) + 0.025 * F32::sin(6.2831855 * self.fRec40[0]) + 0.024390243 * F32::sin(6.2831855 * self.fRec41[0]) + 0.023809524 * F32::sin(6.2831855 * self.fRec42[0]) + 0.023255814 * F32::sin(6.2831855 * self.fRec43[0]) + 0.022727273 * F32::sin(6.2831855 * self.fRec44[0]) + 0.022222223 * F32::sin(6.2831855 * self.fRec45[0]) + 0.02173913 * F32::sin(6.2831855 * self.fRec46[0]) + 0.021276595 * F32::sin(6.2831855 * self.fRec47[0]) + 0.020833334 * F32::sin(6.2831855 * self.fRec48[0]) + 0.020408163 * F32::sin(6.2831855 * self.fRec49[0]) + 0.02 * F32::sin(6.2831855 * self.fRec50[0]) + 0.019607844 * F32::sin(6.2831855 * self.fRec51[0]) + 0.01923077 * F32::sin(6.2831855 * self.fRec52[0]) + 0.018867925 * F32::sin(6.2831855 * self.fRec53[0]) + 0.018518519 * F32::sin(6.2831855 * self.fRec54[0]) + 0.018181818 * F32::sin(6.2831855 * self.fRec55[0]) + 0.017857144 * F32::sin(6.2831855 * self.fRec56[0]) + 0.01754386 * F32::sin(6.2831855 * self.fRec57[0]) + 0.01724138 * F32::sin(6.2831855 * self.fRec58[0]) + 0.016949153 * F32::sin(6.2831855 * self.fRec59[0]) + 0.016666668 * F32::sin(6.2831855 * self.fRec60[0]) + 0.016393442 * F32::sin(6.2831855 * self.fRec61[0]) + 0.016129032 * F32::sin(6.2831855 * self.fRec62[0]) + 0.015873017 * F32::sin(6.2831855 * self.fRec63[0]) + 0.015625 * F32::sin(6.2831855 * self.fRec64[0]) + 0.015384615 * F32::sin(6.2831855 * self.fRec65[0]) + 0.015151516 * F32::sin(6.2831855 * self.fRec66[0]) + 0.014925373 * F32::sin(6.2831855 * self.fRec67[0]) + 0.014705882 * F32::sin(6.2831855 * self.fRec68[0]) + 0.014492754 * F32::sin(6.2831855 * self.fRec69[0]) + 0.014285714 * F32::sin(6.2831855 * self.fRec70[0]) + 0.014084507 * F32::sin(6.2831855 * self.fRec71[0]) + 0.013888889 * F32::sin(6.2831855 * self.fRec72[0]) + 0.01369863 * F32::sin(6.2831855 * self.fRec73[0]) + 0.013513514 * F32::sin(6.2831855 * self.fRec74[0]) + 0.013333334 * F32::sin(6.2831855 * self.fRec75[0]) + 0.013157895 * F32::sin(6.2831855 * self.fRec76[0]) + 0.012987013 * F32::sin(6.2831855 * self.fRec77[0]) + 0.012820513 * F32::sin(6.2831855 * self.fRec78[0]) + 0.012658228 * F32::sin(6.2831855 * self.fRec79[0]) + 0.0125 * F32::sin(6.2831855 * self.fRec80[0]) + 0.012345679 * F32::sin(6.2831855 * self.fRec81[0]) + 0.0121951215 * F32::sin(6.2831855 * self.fRec82[0]) + 0.012048192 * F32::sin(6.2831855 * self.fRec83[0]) + 0.011904762 * F32::sin(6.2831855 * self.fRec84[0]) + 0.011764706 * F32::sin(6.2831855 * self.fRec85[0]) + 0.011627907 * F32::sin(6.2831855 * self.fRec86[0]) + 0.011494253 * F32::sin(6.2831855 * self.fRec87[0]) + 0.011363637 * F32::sin(6.2831855 * self.fRec88[0]) + 0.011235955 * F32::sin(6.2831855 * self.fRec89[0]) + 0.011111111 * F32::sin(6.2831855 * self.fRec90[0]) + 0.010989011 * F32::sin(6.2831855 * self.fRec91[0]) + 0.010869565 * F32::sin(6.2831855 * self.fRec92[0]) + 0.010752688 * F32::sin(6.2831855 * self.fRec93[0]) + 0.010638298 * F32::sin(6.2831855 * self.fRec94[0]) + 0.010526316 * F32::sin(6.2831855 * self.fRec95[0]) + 0.010416667 * F32::sin(6.2831855 * self.fRec96[0]) + 0.010309278 * F32::sin(6.2831855 * self.fRec97[0]) + 0.010204081 * F32::sin(6.2831855 * self.fRec98[0]) + 0.01010101 * F32::sin(6.2831855 * self.fRec99[0]);
			self.iVec0[1] = self.iVec0[0];
			self.fRec0[1] = self.fRec0[0];
			self.fRec1[1] = self.fRec1[0];
			self.fRec2[1] = self.fRec2[0];
			self.fRec3[1] = self.fRec3[0];
			self.fRec4[1] = self.fRec4[0];
			self.fRec5[1] = self.fRec5[0];
			self.fRec6[1] = self.fRec6[0];
			self.fRec7[1] = self.fRec7[0];
			self.fRec8[1] = self.fRec8[0];
			self.fRec9[1] = self.fRec9[0];
			self.fRec10[1] = self.fRec10[0];
			self.fRec11[1] = self.fRec11[0];
			self.fRec12[1] = self.fRec12[0];
			self.fRec13[1] = self.fRec13[0];
			self.fRec14[1] = self.fRec14[0];
			self.fRec15[1] = self.fRec15[0];
			self.fRec16[1] = self.fRec16[0];
			self.fRec17[1] = self.fRec17[0];
			self.fRec18[1] = self.fRec18[0];
			self.fRec19[1] = self.fRec19[0];
			self.fRec20[1] = self.fRec20[0];
			self.fRec21[1] = self.fRec21[0];
			self.fRec22[1] = self.fRec22[0];
			self.fRec23[1] = self.fRec23[0];
			self.fRec24[1] = self.fRec24[0];
			self.fRec25[1] = self.fRec25[0];
			self.fRec26[1] = self.fRec26[0];
			self.fRec27[1] = self.fRec27[0];
			self.fRec28[1] = self.fRec28[0];
			self.fRec29[1] = self.fRec29[0];
			self.fRec30[1] = self.fRec30[0];
			self.fRec31[1] = self.fRec31[0];
			self.fRec32[1] = self.fRec32[0];
			self.fRec33[1] = self.fRec33[0];
			self.fRec34[1] = self.fRec34[0];
			self.fRec35[1] = self.fRec35[0];
			self.fRec36[1] = self.fRec36[0];
			self.fRec37[1] = self.fRec37[0];
			self.fRec38[1] = self.fRec38[0];
			self.fRec39[1] = self.fRec39[0];
			self.fRec40[1] = self.fRec40[0];
			self.fRec41[1] = self.fRec41[0];
			self.fRec42[1] = self.fRec42[0];
			self.fRec43[1] = self.fRec43[0];
			self.fRec44[1] = self.fRec44[0];
			self.fRec45[1] = self.fRec45[0];
			self.fRec46[1] = self.fRec46[0];
			self.fRec47[1] = self.fRec47[0];
			self.fRec48[1] = self.fRec48[0];
			self.fRec49[1] = self.fRec49[0];
			self.fRec50[1] = self.fRec50[0];
			self.fRec51[1] = self.fRec51[0];
			self.fRec52[1] = self.fRec52[0];
			self.fRec53[1] = self.fRec53[0];
			self.fRec54[1] = self.fRec54[0];
			self.fRec55[1] = self.fRec55[0];
			self.fRec56[1] = self.fRec56[0];
			self.fRec57[1] = self.fRec57[0];
			self.fRec58[1] = self.fRec58[0];
			self.fRec59[1] = self.fRec59[0];
			self.fRec60[1] = self.fRec60[0];
			self.fRec61[1] = self.fRec61[0];
			self.fRec62[1] = self.fRec62[0];
			self.fRec63[1] = self.fRec63[0];
			self.fRec64[1] = self.fRec64[0];
			self.fRec65[1] = self.fRec65[0];
			self.fRec66[1] = self.fRec66[0];
			self.fRec67[1] = self.fRec67[0];
			self.fRec68[1] = self.fRec68[0];
			self.fRec69[1] = self.fRec69[0];
			self.fRec70[1] = self.fRec70[0];
			self.fRec71[1] = self.fRec71[0];
			self.fRec72[1] = self.fRec72[0];
			self.fRec73[1] = self.fRec73[0];
			self.fRec74[1] = self.fRec74[0];
			self.fRec75[1] = self.fRec75[0];
			self.fRec76[1] = self.fRec76[0];
			self.fRec77[1] = self.fRec77[0];
			self.fRec78[1] = self.fRec78[0];
			self.fRec79[1] = self.fRec79[0];
			self.fRec80[1] = self.fRec80[0];
			self.fRec81[1] = self.fRec81[0];
			self.fRec82[1] = self.fRec82[0];
			self.fRec83[1] = self.fRec83[0];
			self.fRec84[1] = self.fRec84[0];
			self.fRec85[1] = self.fRec85[0];
			self.fRec86[1] = self.fRec86[0];
			self.fRec87[1] = self.fRec87[0];
			self.fRec88[1] = self.fRec88[0];
			self.fRec89[1] = self.fRec89[0];
			self.fRec90[1] = self.fRec90[0];
			self.fRec91[1] = self.fRec91[0];
			self.fRec92[1] = self.fRec92[0];
			self.fRec93[1] = self.fRec93[0];
			self.fRec94[1] = self.fRec94[0];
			self.fRec95[1] = self.fRec95[0];
			self.fRec96[1] = self.fRec96[0];
			self.fRec97[1] = self.fRec97[0];
			self.fRec98[1] = self.fRec98[0];
			self.fRec99[1] = self.fRec99[0];
		}
		
	}

}

impl FaustDsp for mydsp {
	type T = FaustFloat;
	fn new() -> Self where Self: Sized {
		Self::new()
	}
	fn metadata(&self, m: &mut dyn Meta) {
		self.metadata(m)
	}
	fn get_sample_rate(&self) -> i32 {
		self.get_sample_rate()
	}
	fn get_num_inputs(&self) -> i32 {
		FAUST_INPUTS as i32
	}
	fn get_num_outputs(&self) -> i32 {
		FAUST_OUTPUTS as i32
	}
	fn class_init(sample_rate: i32) where Self: Sized {
		Self::class_init(sample_rate);
	}
	fn instance_reset_params(&mut self) {
		self.instance_reset_params()
	}
	fn instance_clear(&mut self) {
		self.instance_clear()
	}
	fn instance_constants(&mut self, sample_rate: i32) {
		self.instance_constants(sample_rate)
	}
	fn instance_init(&mut self, sample_rate: i32) {
		self.instance_init(sample_rate)
	}
	fn init(&mut self, sample_rate: i32) {
		self.init(sample_rate)
	}
	fn build_user_interface(&self, ui_interface: &mut dyn UI<Self::T>) {
		self.build_user_interface(ui_interface)
	}
	fn build_user_interface_static(ui_interface: &mut dyn UI<Self::T>) where Self: Sized {
		Self::build_user_interface_static(ui_interface);
	}
	fn get_param(&self, param: ParamIndex) -> Option<Self::T> {
		self.get_param(param)
	}
	fn set_param(&mut self, param: ParamIndex, value: Self::T) {
		self.set_param(param, value)
	}
	fn compute(&mut self, count: i32, inputs: &[&[Self::T]], outputs: &mut [&mut [Self::T]]) {
		self.compute(count as usize, inputs, outputs)
	}
}

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;
