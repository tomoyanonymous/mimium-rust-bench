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
extern crate portaudio;
use portaudio as pa;
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

#[cfg_attr(feature = "default-boxed", derive(default_boxed::DefaultBoxed))]
#[repr(C)]
pub struct mydsp {
	iVec0: [i32;2],
	fSampleRate: i32,
	fConst0: F32,
	fConst1: F32,
	fRec0: [F32;2],
	fConst2: F32,
	fRec1: [F32;2],
	fConst3: F32,
	fRec2: [F32;2],
	fConst4: F32,
	fRec3: [F32;2],
	fConst5: F32,
	fRec4: [F32;2],
	fConst6: F32,
	fRec5: [F32;2],
	fConst7: F32,
	fRec6: [F32;2],
	fConst8: F32,
	fRec7: [F32;2],
	fConst9: F32,
	fRec8: [F32;2],
	fRec9: [F32;2],
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
			fSampleRate: 0,
			fConst0: 0.0,
			fConst1: 0.0,
			fRec0: [0.0;2],
			fConst2: 0.0,
			fRec1: [0.0;2],
			fConst3: 0.0,
			fRec2: [0.0;2],
			fConst4: 0.0,
			fRec3: [0.0;2],
			fConst5: 0.0,
			fRec4: [0.0;2],
			fConst6: 0.0,
			fRec5: [0.0;2],
			fConst7: 0.0,
			fRec6: [0.0;2],
			fConst8: 0.0,
			fRec7: [0.0;2],
			fConst9: 0.0,
			fRec8: [0.0;2],
			fRec9: [0.0;2],
		}
	}
	pub fn metadata(&self, m: &mut dyn Meta) { 
		m.declare("compile_options", r"-a /usr/local/share/faust/rust/portaudio-float.rs -lang rust -ct 1 -es 1 -mcd 16 -mdd 1024 -mdy 33 -single -ftz 0");
		m.declare("filename", r"osc.dsp");
		m.declare("maths.lib/author", r"GRAME");
		m.declare("maths.lib/copyright", r"GRAME");
		m.declare("maths.lib/license", r"LGPL with exception");
		m.declare("maths.lib/name", r"Faust Math Library");
		m.declare("maths.lib/version", r"2.9.0");
		m.declare("name", r"osc");
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
	}
	pub fn instance_constants(&mut self, sample_rate: i32) {
		// Obtaining locks on 0 static var(s)
		self.fSampleRate = sample_rate;
		self.fConst0 = F32::min(1.92e+05, F32::max(1.0, (self.fSampleRate) as F32));
		self.fConst1 = 9e+03 / self.fConst0;
		self.fConst2 = 8e+03 / self.fConst0;
		self.fConst3 = 7e+03 / self.fConst0;
		self.fConst4 = 6e+03 / self.fConst0;
		self.fConst5 = 5e+03 / self.fConst0;
		self.fConst6 = 4e+03 / self.fConst0;
		self.fConst7 = 3e+03 / self.fConst0;
		self.fConst8 = 2e+03 / self.fConst0;
		self.fConst9 = 1e+03 / self.fConst0;
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
		ui_interface.open_vertical_box("osc");
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
			let mut fTemp1: F32 = (if iTemp0 != 0 {0.0} else {self.fConst1 + self.fRec0[1]});
			self.fRec0[0] = fTemp1 - F32::floor(fTemp1);
			let mut fTemp2: F32 = (if iTemp0 != 0 {0.0} else {self.fConst2 + self.fRec1[1]});
			self.fRec1[0] = fTemp2 - F32::floor(fTemp2);
			let mut fTemp3: F32 = (if iTemp0 != 0 {0.0} else {self.fConst3 + self.fRec2[1]});
			self.fRec2[0] = fTemp3 - F32::floor(fTemp3);
			let mut fTemp4: F32 = (if iTemp0 != 0 {0.0} else {self.fConst4 + self.fRec3[1]});
			self.fRec3[0] = fTemp4 - F32::floor(fTemp4);
			let mut fTemp5: F32 = (if iTemp0 != 0 {0.0} else {self.fConst5 + self.fRec4[1]});
			self.fRec4[0] = fTemp5 - F32::floor(fTemp5);
			let mut fTemp6: F32 = (if iTemp0 != 0 {0.0} else {self.fConst6 + self.fRec5[1]});
			self.fRec5[0] = fTemp6 - F32::floor(fTemp6);
			let mut fTemp7: F32 = (if iTemp0 != 0 {0.0} else {self.fConst7 + self.fRec6[1]});
			self.fRec6[0] = fTemp7 - F32::floor(fTemp7);
			let mut fTemp8: F32 = (if iTemp0 != 0 {0.0} else {self.fConst8 + self.fRec7[1]});
			self.fRec7[0] = fTemp8 - F32::floor(fTemp8);
			let mut fTemp9: F32 = (if iTemp0 != 0 {0.0} else {self.fConst9 + self.fRec8[1]});
			self.fRec8[0] = fTemp9 - F32::floor(fTemp9);
			let mut fTemp10: F32 = (if iTemp0 != 0 {0.0} else {self.fRec9[1]});
			self.fRec9[0] = fTemp10 - F32::floor(fTemp10);
			*output0 = F32::sin(6.2831855 * self.fRec9[0]) + F32::sin(6.2831855 * self.fRec8[0]) + 0.5 * F32::sin(6.2831855 * self.fRec7[0]) + 0.33333334 * F32::sin(6.2831855 * self.fRec6[0]) + 0.25 * F32::sin(6.2831855 * self.fRec5[0]) + 0.2 * F32::sin(6.2831855 * self.fRec4[0]) + 0.16666667 * F32::sin(6.2831855 * self.fRec3[0]) + 0.14285715 * F32::sin(6.2831855 * self.fRec2[0]) + 0.125 * F32::sin(6.2831855 * self.fRec1[0]) + 0.11111111 * F32::sin(6.2831855 * self.fRec0[0]);
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

fn main() {
    run().unwrap()
}

fn run() -> Result<(), pa::Error> {

    let pa = pa::PortAudio::new()?;

    // Allocation DSP on the heap
    let mut dsp;
    #[cfg(feature = "default-boxed")]
    {
        use default_boxed::DefaultBoxed;
        dsp = mydsp::default_boxed();
    }

    #[cfg(not(feature = "default-boxed"))]
    {
        dsp = Box::new(mydsp::new());
    }

    println!("Faust Rust code running with Portaudio: sample-rate = {} buffer-size = {}", SAMPLE_RATE, FRAMES_PER_BUFFER);

    //Create a input/output stream with the same number of input and output channels
    const INTERLEAVED: bool = false;// We want NON interleaved streams
    let input_device = pa.default_input_device()?;
    let output_device = pa.default_output_device()?;
    let input_latency = pa.device_info(input_device)?.default_low_input_latency;
    let output_latency = pa.device_info(output_device)?.default_low_input_latency;

    let in_params = pa::StreamParameters::new(input_device, CHANNELS, INTERLEAVED, input_latency);
    let out_params = pa::StreamParameters::new(output_device, CHANNELS, INTERLEAVED, output_latency);
    let settings = pa::DuplexStreamSettings::new(in_params, out_params, SAMPLE_RATE, FRAMES_PER_BUFFER);
    
    //This would have been interleaved:
    //let mut settings = try!(pa.default_duplex_stream_settings(CHANNELS, CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER));

    println!("get_num_inputs: {}", dsp.get_num_inputs());
    println!("get_num_outputs: {}", dsp.get_num_outputs());

    // Init DSP with a given SR
    dsp.init(SAMPLE_RATE as i32);

    //settings.flags = pa::stream_flags::CLIP_OFF;

    // This routine will be called by the PortAudio engine when audio is needed. It may called at
    // interrupt level on some machines so don't do anything that could mess up the system like
    // dynamic resource allocation or IO.
    let callback = move |pa::DuplexStreamCallbackArgs { in_buffer, out_buffer, frames, time, .. } : pa::DuplexStreamCallbackArgs<f32, f32>| {
        let out_buffr: &mut [*mut f32];
        let in_buffr: & [*const f32];
        //rust-portaudio does not support non-interleaved audio out of the box (but portaudio does)
        unsafe {

            let in_buffer: *const *const f32 = ::std::mem::transmute(in_buffer.get_unchecked(0));
            in_buffr = ::std::slice::from_raw_parts(in_buffer, CHANNELS as usize);
            let input0 = ::std::slice::from_raw_parts(in_buffr[0], frames);
            let input1 = ::std::slice::from_raw_parts(in_buffr[1], frames);

            let out_buffer: *mut *mut f32 = ::std::mem::transmute(out_buffer.get_unchecked_mut(0));
            out_buffr = ::std::slice::from_raw_parts_mut(out_buffer, CHANNELS as usize);
            let output0 = ::std::slice::from_raw_parts_mut(out_buffr[0], frames);
            let output1 = ::std::slice::from_raw_parts_mut(out_buffr[1], frames);

            let inputs = &[input0, input1];
            let outputs = &mut [output0, output1];

            dsp.compute(frames as usize, inputs, outputs);
        }
        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;

    // Wait for user input to quit
    println!("Press enter/return to quit...");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    stream.stop()?;
    stream.close()?;

    Ok(())
}
