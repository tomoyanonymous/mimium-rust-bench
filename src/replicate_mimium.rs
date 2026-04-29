use std::convert::TryInto;

pub type Word = u64;
pub type mmmfloat = f32;

trait MmmFloatWord: Sized {
    fn to_word(self) -> Word;
    fn from_word(word: Word) -> Self;
}
impl MmmFloatWord for f64 {
    #[inline(always)] fn to_word(self) -> Word { self.to_bits() }
    #[inline(always)] fn from_word(word: Word) -> Self { f64::from_bits(word) }
}
impl MmmFloatWord for f32 {
    #[inline(always)] fn to_word(self) -> Word { self.to_bits() as Word }
    #[inline(always)] fn from_word(word: Word) -> Self { f32::from_bits(word as u32) }
}

#[inline(always)]
fn f64_to_word(value: mmmfloat) -> Word { value.to_word() }
#[inline(always)]
fn word_to_f64(value: Word) -> mmmfloat { mmmfloat::from_word(value) }
#[inline(always)]
fn i64_to_word(value: i64) -> Word { u64::from_ne_bytes(value.to_ne_bytes()) }
#[inline(always)]
fn word_to_i64(value: Word) -> i64 { i64::from_ne_bytes(value.to_ne_bytes()) }
#[inline(always)]
fn copy_words<const N: usize>(slice: &[Word]) -> Result<[Word; N], String> {
    slice
        .try_into()
        .map_err(|_| format!("expected {} words, got {}", N, slice.len()))
}
#[inline(always)]
fn vec_to_words<const N: usize>(words: Vec<Word>) -> Result<[Word; N], String> {
    copy_words::<N>(&words)
}
#[inline(always)]
fn truthy(value: Word) -> bool { word_to_f64(value) > 0.0 }
const FUNCTION_HANDLE_TAG: Word = 1 << 63;
const CLOSURE_HANDLE_TAG: Word = 1 << 62;
const MEMORY_HANDLE_TAG: Word = 1 << 61;
fn encode_function(index: usize) -> Word { FUNCTION_HANDLE_TAG | index as Word }
fn encode_closure(index: usize) -> Word { CLOSURE_HANDLE_TAG | index as Word }
fn encode_memory(index: usize) -> Word { MEMORY_HANDLE_TAG | index as Word }
fn decode_memory(handle: Word) -> Option<usize> {
    if handle & MEMORY_HANDLE_TAG != 0 {
        Some((handle & !MEMORY_HANDLE_TAG) as usize)
    } else {
        None
    }
}
fn decode_function(handle: Word) -> Option<usize> {
    if handle & FUNCTION_HANDLE_TAG != 0 {
        Some((handle & !FUNCTION_HANDLE_TAG) as usize)
    } else {
        Some(handle as usize)
    }
}

fn decode_closure(handle: Word) -> Option<usize> {
    if handle & FUNCTION_HANDLE_TAG != 0 {
        None
    } else if handle & CLOSURE_HANDLE_TAG != 0 {
        Some((handle & !CLOSURE_HANDLE_TAG) as usize)
    } else {
        None
    }
}

fn parse_specialized_arity(name: &str, prefix: &str, default: usize) -> Option<usize> {
    name.strip_prefix(prefix)
        .map(|suffix| suffix.parse::<usize>().ok())
        .unwrap_or(Some(default))
}

pub trait MimiumHost {
    fn call_ext(&mut self, name: &str, args: &[Word], ret_words: usize) -> Result<Vec<Word>, String>;

    fn current_time(&mut self) -> mmmfloat {
        0.0
    }

    fn sample_rate(&mut self) -> mmmfloat {
        48_000.0
    }
}

#[derive(Default)]
pub struct PanicHost;

impl MimiumHost for PanicHost {
    fn call_ext(&mut self, name: &str, _args: &[Word], _ret_words: usize) -> Result<Vec<Word>, String> {
        Err(format!("external function '{}' is not available in the generated Rust host", name))
    }
}

#[derive(Clone, Default)]
struct StateStorage {
    pos: usize,
    rawdata: Vec<Word>,
}

impl StateStorage {
    fn new(size: usize) -> Self {
        Self {
            pos: 0,
            rawdata: vec![0; size],
        }
    }

    fn ensure(&mut self, size: usize) {
        let needed = self.pos.saturating_add(size);
        if self.rawdata.len() < needed {
            self.rawdata.resize(needed, 0);
        }
    }

    #[inline(always)]
    fn push_pos(&mut self, offset: usize) {
        self.pos += offset;
    }

    #[inline(always)]
    fn pop_pos(&mut self, offset: usize) {
        self.pos -= offset;
    }

    fn get_state(&mut self, size: usize) -> Vec<Word> {
        self.ensure(size);
        self.rawdata[self.pos..self.pos + size].to_vec()
    }

    #[inline(always)]
    fn get_state_slice(&self, size: usize) -> &[Word] {
        debug_assert!(self.pos + size <= self.rawdata.len(), "state read out of bounds: pos={} size={} len={}", self.pos, size, self.rawdata.len());
        unsafe { self.rawdata.get_unchecked(self.pos..self.pos + size) }
    }

    #[inline(always)]
    fn get_state_word(&self) -> Word {
        debug_assert!(self.pos < self.rawdata.len(), "state read out of bounds: pos={} len={}", self.pos, self.rawdata.len());
        unsafe { *self.rawdata.get_unchecked(self.pos) }
    }

    #[inline(always)]
    fn set_state(&mut self, src: &[Word], size: usize) {
        debug_assert!(self.pos + size <= self.rawdata.len(), "state write out of bounds: pos={} size={} len={}", self.pos, size, self.rawdata.len());
        unsafe { self.rawdata.get_unchecked_mut(self.pos..self.pos + size) }.copy_from_slice(&src[..size]);
    }

    #[inline(always)]
    fn set_state_word(&mut self, src: Word) {
        debug_assert!(self.pos < self.rawdata.len(), "state write out of bounds: pos={} len={}", self.pos, self.rawdata.len());
        unsafe { *self.rawdata.get_unchecked_mut(self.pos) = src; }
    }

    #[inline(always)]
    fn mem(&mut self, src: Word) -> Word {
        debug_assert!(self.pos < self.rawdata.len(), "mem out of bounds: pos={} len={}", self.pos, self.rawdata.len());
        let prev = unsafe { *self.rawdata.get_unchecked(self.pos) };
        unsafe { *self.rawdata.get_unchecked_mut(self.pos) = src; }
        prev
    }

    fn delay(&mut self, input: Word, time_raw: Word, max_len: usize) -> Word {
        let total_words = max_len.saturating_add(2);
        self.ensure(total_words);
        if max_len == 0 {
            return 0;
        }

        let delay_samples = word_to_f64(time_raw)
            .clamp(0.0, max_len.saturating_sub(1) as mmmfloat) as usize;
        let read_slot = self.pos;
        let write_slot = self.pos + 1;
        let data_start = self.pos + 2;
        let write_idx = (self.rawdata[write_slot] as usize) % max_len;
        let read_idx = (write_idx + max_len - delay_samples) % max_len;
        let result = self.rawdata[data_start + read_idx];
        self.rawdata[data_start + write_idx] = input;
        self.rawdata[read_slot] = read_idx as u64;
        self.rawdata[write_slot] = ((write_idx + 1) % max_len) as u64;
        result
    }
}

#[derive(Clone, Default)]
struct Pointer {
    slot: usize,
    offset: usize,
}

#[derive(Default)]
struct MemoryStore {
    slots: Vec<Vec<Word>>,
    ptrs: Vec<Pointer>,
}

impl MemoryStore {
    fn alloc(&mut self, size: usize) -> Word {
        let slot = self.slots.len();
        self.slots.push(vec![0; size]);
        self.ptrs.push(Pointer { slot, offset: 0 });
        encode_memory(self.ptrs.len())
    }

    fn ptr(&self, handle: Word) -> Result<&Pointer, String> {
        let index = decode_memory(handle)
            .and_then(|value| value.checked_sub(1))
            .ok_or_else(|| format!("invalid memory handle {}", handle))?;
        self.ptrs
            .get(index)
            .ok_or_else(|| format!("invalid memory handle {}", handle))
    }

    fn get_element(&mut self, base: Word, tuple_offset: usize) -> Result<Word, String> {
        let pointer = self.ptr(base)?.clone();
        self.ptrs.push(Pointer {
            slot: pointer.slot,
            offset: pointer.offset + tuple_offset,
        });
        Ok(encode_memory(self.ptrs.len()))
    }

    #[inline(always)]
    fn load_word(&self, ptr: Word) -> Result<Word, String> {
        let Some(index) = decode_memory(ptr).and_then(|value| value.checked_sub(1)) else {
            return Ok(ptr);
        };
        let pointer = self
            .ptrs
            .get(index)
            .ok_or_else(|| format!("invalid memory handle {}", ptr))?;
        let slot = self
            .slots
            .get(pointer.slot)
            .ok_or_else(|| format!("invalid memory slot {}", pointer.slot))?;
        slot.get(pointer.offset)
            .copied()
            .ok_or_else(|| format!("load out of bounds: offset={} len={}", pointer.offset, slot.len()))
    }

    #[inline(always)]
    fn load(&self, ptr: Word, size: usize) -> Result<Vec<Word>, String> {
        let Some(index) = decode_memory(ptr).and_then(|value| value.checked_sub(1)) else {
            if size == 1 {
                return Ok(vec![ptr]);
            }
            return Err(format!("invalid memory handle {}", ptr));
        };
        let Some(pointer) = self.ptrs.get(index) else {
            if size == 1 {
                return Ok(vec![ptr]);
            }
            return Err(format!("invalid memory handle {}", ptr));
        };
        let slot = self
            .slots
            .get(pointer.slot)
            .ok_or_else(|| format!("invalid memory slot {}", pointer.slot))?;
        let end = pointer.offset + size;
        if end > slot.len() {
            return Err(format!(
                "load out of bounds: offset={} size={} len={}",
                pointer.offset,
                size,
                slot.len()
            ));
        }
        Ok(slot[pointer.offset..end].to_vec())
    }

    #[inline(always)]
    fn store_word(&mut self, ptr: Word, src: Word) -> Result<(), String> {
        let pointer = self.ptr(ptr)?.clone();
        let slot = self
            .slots
            .get_mut(pointer.slot)
            .ok_or_else(|| format!("invalid memory slot {}", pointer.slot))?;
        let len = slot.len();
        if pointer.offset >= len {
            return Err(format!("store out of bounds: offset={} len={}", pointer.offset, len));
        }
        slot[pointer.offset] = src;
        Ok(())
    }

    #[inline(always)]
    fn store(&mut self, ptr: Word, src: &[Word], size: usize) -> Result<(), String> {
        let pointer = self.ptr(ptr)?.clone();
        let slot = self
            .slots
            .get_mut(pointer.slot)
            .ok_or_else(|| format!("invalid memory slot {}", pointer.slot))?;
        let end = pointer.offset + size;
        if end > slot.len() {
            return Err(format!(
                "store out of bounds: offset={} size={} len={}",
                pointer.offset,
                size,
                slot.len()
            ));
        }
        slot[pointer.offset..end].copy_from_slice(&src[..size]);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct ArrayObject {
    elem_size_words: usize,
    data: Vec<Word>,
}

#[derive(Default)]
struct ArrayStorage {
    arrays: Vec<ArrayObject>,
}

impl ArrayStorage {
    fn alloc_array(&mut self, len: usize, elem_size_words: usize) -> Word {
        self.arrays.push(ArrayObject {
            elem_size_words,
            data: vec![0; len.saturating_mul(elem_size_words)],
        });
        self.arrays.len() as Word
    }

    fn alloc_array_with_data(&mut self, data: Vec<Word>, elem_size_words: usize) -> Word {
        self.arrays.push(ArrayObject {
            elem_size_words,
            data,
        });
        self.arrays.len() as Word
    }

    fn get(&self, handle: Word) -> Result<&ArrayObject, String> {
        let index = handle
            .checked_sub(1)
            .ok_or_else(|| "invalid array handle 0".to_string())? as usize;
        self.arrays
            .get(index)
            .ok_or_else(|| format!("invalid array handle {}", handle))
    }

    fn get_mut(&mut self, handle: Word) -> Result<&mut ArrayObject, String> {
        let index = handle
            .checked_sub(1)
            .ok_or_else(|| "invalid array handle 0".to_string())? as usize;
        self.arrays
            .get_mut(index)
            .ok_or_else(|| format!("invalid array handle {}", handle))
    }
}

#[derive(Clone)]
struct ClosureObject {
    function: Word,
    upvalues: Vec<Word>,
    indirect: Vec<bool>,
    state_storage: StateStorage,
}

#[derive(Default)]
struct ClosureStorage {
    closures: Vec<ClosureObject>,
}

impl ClosureStorage {
    fn alloc(
        &mut self,
        function: Word,
        upvalues: Vec<Word>,
        indirect: Vec<bool>,
        state_size: usize,
    ) -> Result<Word, String> {
        if upvalues.len() != indirect.len() {
            return Err(format!(
                "closure upvalue metadata mismatch: {} values, {} flags",
                upvalues.len(),
                indirect.len()
            ));
        }
        let index = self.closures.len();
        self.closures.push(ClosureObject {
            function,
            upvalues,
            indirect,
            state_storage: StateStorage::new(state_size),
        });
        Ok(encode_closure(index))
    }

    fn get(&self, handle: Word) -> Result<&ClosureObject, String> {
        let index = decode_closure(handle)
            .ok_or_else(|| format!("invalid closure handle {}", handle))?;
        self.closures
            .get(index)
            .ok_or_else(|| format!("invalid closure handle {}", handle))
    }

    fn get_mut(&mut self, handle: Word) -> Result<&mut ClosureObject, String> {
        let index = decode_closure(handle)
            .ok_or_else(|| format!("invalid closure handle {}", handle))?;
        self.closures
            .get_mut(index)
            .ok_or_else(|| format!("invalid closure handle {}", handle))
    }
}

pub struct MimiumProgram<H: MimiumHost = PanicHost> {
    pub host: H,
    globals: Vec<Vec<Word>>,
    function_states: Vec<StateStorage>,
    current_function_state: Option<usize>,
    state_storage_stack: Vec<Word>,
    memory: MemoryStore,
    closures: ClosureStorage,
    arrays: ArrayStorage,
    strings: Vec<String>,
}

impl MimiumProgram<PanicHost> {
    pub fn new() -> Self {
        Self::with_host(PanicHost)
    }
}

impl<H: MimiumHost> MimiumProgram<H> {
    pub fn with_host(host: H) -> Self {
        Self {
            host,
            globals: vec![

            ],
            function_states: vec![
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(0),
                StateStorage::new(1),
                StateStorage::new(11),

            ],
            current_function_state: None,
            state_storage_stack: Vec::new(),
            memory: MemoryStore::default(),
            closures: ClosureStorage::default(),
            arrays: ArrayStorage::default(),
            strings: Vec::new(),
        }
    }

    pub fn call_dsp(&mut self, args: &[Word]) -> Result<Vec<Word>, String> {
        let previous_function_state = self.current_function_state;
        self.current_function_state = Some(28);
        let result = self.dispatch_dsp(args);
        self.current_function_state = previous_function_state;
        Ok(result)
    }

    pub fn call_dsp_buffer(&mut self, input: &[mmmfloat], output: &mut [mmmfloat], frames: usize) -> Result<(), String> {
        if !input.is_empty() {
            return Err(format!("expected 0 input samples for {} dsp frames, got {}", frames, input.len()));
        }
        let expected_output_len = frames.saturating_mul(2usize);
        if output.len() != expected_output_len {
            return Err(format!("expected {} output samples for {} dsp frames, got {}", expected_output_len, frames, output.len()));
        }
        let previous_function_state = self.current_function_state;
        self.current_function_state = Some(28);
        for frame in 0..frames {
            let frame_output_start = frame * 2usize;
            let mut result_words = [0u64; 2];
            self.dsp(&mut result_words);
            output[frame_output_start + 0usize] = word_to_f64(result_words[0]);
            output[frame_output_start + 1usize] = word_to_f64(result_words[1]);
        }
        self.current_function_state = previous_function_state;
        Ok(())
    }



    fn call_function_handle(&mut self, handle: Word, args: &[Word]) -> Vec<Word> {
        self.call_function_handle_with_memory(handle, args)
    }

    #[inline(always)]
    fn get_current_statestorage(&mut self) -> &mut StateStorage {
        if let Some(&closure_handle) = self.state_storage_stack.last() {
            let index = unsafe { decode_closure(closure_handle).unwrap_unchecked() };
            unsafe { &mut self.closures.closures.get_unchecked_mut(index).state_storage }
        } else {
            let function_index = unsafe { self.current_function_state.unwrap_unchecked() };
            unsafe { self.function_states.get_unchecked_mut(function_index) }
        }
    }

    fn get_current_closure(&self) -> Option<Word> {
        self.state_storage_stack.last().copied()
    }

    fn call_ext(&mut self, name: &str, args: &[Word], ret_words: usize) -> Result<Vec<Word>, String> {
        match name {
            "min" => {
                let lhs = word_to_f64(*args.get(0).ok_or_else(|| "min expects 2 args".to_string())?);
                let rhs = word_to_f64(*args.get(1).ok_or_else(|| "min expects 2 args".to_string())?);
                Ok(vec![f64_to_word(lhs.min(rhs))])
            }
            "max" => {
                let lhs = word_to_f64(*args.get(0).ok_or_else(|| "max expects 2 args".to_string())?);
                let rhs = word_to_f64(*args.get(1).ok_or_else(|| "max expects 2 args".to_string())?);
                Ok(vec![f64_to_word(lhs.max(rhs))])
            }
            "probe" => Ok(args.first().copied().into_iter().collect()),
            "probeln" => Ok(args.first().copied().into_iter().collect()),
            "len" => {
                let handle = *args.get(0).ok_or_else(|| "len expects 1 arg".to_string())?;
                if handle == 0 {
                    Ok(vec![f64_to_word(0.0)])
                } else {
                    let array = self.arrays.get(handle)?;
                    Ok(vec![f64_to_word(array.data.len() as mmmfloat)])
                }
            }
            _ if name == "split_head" || name.starts_with("split_head$arity") => {
                let elem_words = parse_specialized_arity(name, "split_head$arity", 1)
                    .ok_or_else(|| format!("invalid split_head specialization: {}", name))?;
                let handle = *args.get(0).ok_or_else(|| format!("{} expects 1 arg", name))?;
                if handle == 0 {
                    let mut result = vec![0; elem_words];
                    result.push(0);
                    return Ok(result);
                }
                let array = self.arrays.get(handle)?.clone();
                if array.data.len() < elem_words {
                    return Err(format!("{}: array shorter than one element", name));
                }
                if array.data.len() % elem_words != 0 {
                    return Err(format!(
                        "{}: array length {} is not divisible by elem_words {}",
                        name,
                        array.data.len(),
                        elem_words
                    ));
                }
                let head_words = array.data[..elem_words].to_vec();
                let rest_data = array.data[elem_words..].to_vec();
                let rest_handle = self
                    .arrays
                    .alloc_array_with_data(rest_data, array.elem_size_words);
                let mut result = head_words;
                result.push(rest_handle);
                Ok(result)
            }
            _ if name == "split_tail" || name.starts_with("split_tail$arity") => {
                let elem_words = parse_specialized_arity(name, "split_tail$arity", 1)
                    .ok_or_else(|| format!("invalid split_tail specialization: {}", name))?;
                let handle = *args.get(0).ok_or_else(|| format!("{} expects 1 arg", name))?;
                if handle == 0 {
                    let mut result = vec![0];
                    result.resize(elem_words + 1, 0);
                    return Ok(result);
                }
                let array = self.arrays.get(handle)?.clone();
                if array.data.len() < elem_words {
                    return Err(format!("{}: array shorter than one element", name));
                }
                if array.data.len() % elem_words != 0 {
                    return Err(format!(
                        "{}: array length {} is not divisible by elem_words {}",
                        name,
                        array.data.len(),
                        elem_words
                    ));
                }
                let tail_start = array.data.len() - elem_words;
                let tail_words = array.data[tail_start..].to_vec();
                let rest_data = array.data[..tail_start].to_vec();
                let rest_handle = self
                    .arrays
                    .alloc_array_with_data(rest_data, array.elem_size_words);
                let mut result = vec![rest_handle];
                result.extend_from_slice(&tail_words);
                Ok(result)
            }
            _ if name == "prepend" || name.starts_with("prepend$arity") => {
                let elem_words = parse_specialized_arity(name, "prepend$arity", 1)
                    .ok_or_else(|| format!("invalid prepend specialization: {}", name))?;
                let handle = *args
                    .get(elem_words)
                    .ok_or_else(|| format!("{} expects element + array args", name))?;
                let mut data = args[..elem_words].to_vec();
                if handle != 0 {
                    let array = self.arrays.get(handle)?.clone();
                    if array.elem_size_words != elem_words {
                        return Err(format!(
                            "{}: elem size mismatch, expected {} got {}",
                            name, elem_words, array.elem_size_words
                        ));
                    }
                    data.extend_from_slice(&array.data);
                }
                Ok(vec![self.arrays.alloc_array_with_data(data, elem_words)])
            }
            _ if name == "append" || name.starts_with("append$arity") => {
                let elem_words = parse_specialized_arity(name, "append$arity", 1)
                    .ok_or_else(|| format!("invalid append specialization: {}", name))?;
                let handle = *args
                    .first()
                    .ok_or_else(|| format!("{} expects array + element args", name))?;
                let mut data = if handle == 0 {
                    Vec::new()
                } else {
                    let array = self.arrays.get(handle)?.clone();
                    if array.elem_size_words != elem_words {
                        return Err(format!(
                            "{}: elem size mismatch, expected {} got {}",
                            name, elem_words, array.elem_size_words
                        ));
                    }
                    array.data
                };
                data.extend_from_slice(&args[1..1 + elem_words]);
                Ok(vec![self.arrays.alloc_array_with_data(data, elem_words)])
            }
            _ => self.host.call_ext(name, args, ret_words),
        }
    }

    fn alloc_string(&mut self, value: &str) -> Word {
        self.strings.push(value.to_string());
        self.strings.len() as Word
    }

    fn call_function_handle_with_memory(
        &mut self,
        handle: Word,
        args: &[Word],
    ) -> Vec<Word> {
        let previous_function_state = self.current_function_state;
        let (dispatch_handle, current_closure) = if decode_closure(handle).is_some() {
            let function = self
                .closures
                .get(handle)
                .unwrap_or_else(|err| unreachable!("{err}"))
                .function;
            self.state_storage_stack.push(handle);
            (function, Some(handle))
        } else {
            (handle, None)
        };

        if let Some(function_index) = decode_function(dispatch_handle) {
            self.current_function_state = Some(function_index);
        }

        let result = match decode_function(dispatch_handle) {
            Some(0) => {
                let result = self.dispatch__mimium_global(args);
                result
            },
            Some(1) => {
                let result = self.dispatch_math_PI(args);
                result
            },
            Some(2) => {
                let result = self.dispatch_math_E(args);
                result
            },
            Some(3) => {
                let result = self.dispatch_math_exp(args);
                result
            },
            Some(4) => {
                let result = self.dispatch_math_log2(args);
                result
            },
            Some(5) => {
                let result = self.dispatch_math_log10(args);
                result
            },
            Some(6) => {
                let result = self.dispatch_osc_phasor_zero(args);
                result
            },
            Some(7) => {
                let result = self.dispatch_osc_phasor(args);
                result
            },
            Some(8) => {
                let result = self.dispatch___default_7_phase_shift(args);
                result
            },
            Some(9) => {
                let result = self.dispatch_osc_lfsaw(args);
                result
            },
            Some(10) => {
                let result = self.dispatch___default_9_phase(args);
                result
            },
            Some(11) => {
                let result = self.dispatch_osc_saw(args);
                result
            },
            Some(12) => {
                let result = self.dispatch___default_11_phase(args);
                result
            },
            Some(13) => {
                let result = self.dispatch_osc_tri(args);
                result
            },
            Some(14) => {
                let result = self.dispatch___default_13_phase(args);
                result
            },
            Some(15) => {
                let result = self.dispatch_osc_lftri(args);
                result
            },
            Some(16) => {
                let result = self.dispatch___default_15_phase(args);
                result
            },
            Some(17) => {
                let result = self.dispatch_osc_rect(args);
                result
            },
            Some(18) => {
                let result = self.dispatch___default_17_phase(args);
                result
            },
            Some(19) => {
                let result = self.dispatch___default_17_duty(args);
                result
            },
            Some(20) => {
                let result = self.dispatch_osc_lfrect(args);
                result
            },
            Some(21) => {
                let result = self.dispatch___default_20_phase(args);
                result
            },
            Some(22) => {
                let result = self.dispatch___default_20_duty(args);
                result
            },
            Some(23) => {
                let result = self.dispatch_osc_sinwave(args);
                result
            },
            Some(24) => {
                let result = self.dispatch___default_23_phase(args);
                result
            },
            Some(25) => {
                let result = self.dispatch_osc_lfsinwave(args);
                result
            },
            Some(26) => {
                let result = self.dispatch___default_25_phase(args);
                result
            },
            Some(27) => {
                let result = self.dispatch_osc(args);
                result
            },
            Some(28) => {
                let result = self.dispatch_dsp(args);
                result
            },

            Some(index) => unreachable!("unknown function handle {}", index),
            None => unreachable!("unsupported callable handle {}", handle),
        };

        if current_closure.is_some() {
            self.state_storage_stack
                .pop()
                .unwrap_or_else(|| unreachable!("closure state stack underflow"));
        }
        self.current_function_state = previous_function_state;
        result
    }

    fn load_upvalue(
        &self,
        closure_handle: Word,
        index: usize,
        size: usize,
    ) -> Result<Vec<Word>, String> {
        let closure = self.closures.get(closure_handle)?;
        let value = *closure
            .upvalues
            .get(index)
            .ok_or_else(|| format!("invalid upvalue index {}", index))?;
        let indirect = *closure
            .indirect
            .get(index)
            .ok_or_else(|| format!("missing upvalue metadata {}", index))?;

        if indirect {
            self.memory.load(value, size)
        } else if size == 1 {
            Ok(vec![value])
        } else {
            Err(format!(
                "direct upvalue {} does not support {} words in the initial Rust backend",
                index, size
            ))
        }
    }

    fn load_upvalue_word(&self, closure_handle: Word, index: usize) -> Result<Word, String> {
        let closure = self.closures.get(closure_handle)?;
        let value = *closure
            .upvalues
            .get(index)
            .ok_or_else(|| format!("invalid upvalue index {}", index))?;
        let indirect = *closure
            .indirect
            .get(index)
            .ok_or_else(|| format!("missing upvalue metadata {}", index))?;

        if indirect {
            self.memory.load_word(value)
        } else {
            Ok(value)
        }
    }

    fn store_upvalue(
        &mut self,
        closure_handle: Word,
        index: usize,
        src: &[Word],
        size: usize,
    ) -> Result<(), String> {
        let indirect = *self
            .closures
            .get(closure_handle)?
            .indirect
            .get(index)
            .ok_or_else(|| format!("missing upvalue metadata {}", index))?;

        if indirect {
            let ptr = *self
                .closures
                .get(closure_handle)?
                .upvalues
                .get(index)
                .ok_or_else(|| format!("invalid upvalue index {}", index))?;
            self.memory.store(ptr, src, size)
        } else if size == 1 {
            let slot = self
                .closures
                .get_mut(closure_handle)?
                .upvalues
                .get_mut(index)
                .ok_or_else(|| format!("invalid upvalue index {}", index))?;
            *slot = src[0];
            Ok(())
        } else {
            Err(format!(
                "direct upvalue {} does not support {} words in the initial Rust backend",
                index, size
            ))
        }
    }

    fn store_upvalue_word(
        &mut self,
        closure_handle: Word,
        index: usize,
        src: Word,
    ) -> Result<(), String> {
        let indirect = *self
            .closures
            .get(closure_handle)?
            .indirect
            .get(index)
            .ok_or_else(|| format!("missing upvalue metadata {}", index))?;

        if indirect {
            let ptr = *self
                .closures
                .get(closure_handle)?
                .upvalues
                .get(index)
                .ok_or_else(|| format!("invalid upvalue index {}", index))?;
            self.memory.store_word(ptr, src)
        } else {
            let slot = self
                .closures
                .get_mut(closure_handle)?
                .upvalues
                .get_mut(index)
                .ok_or_else(|| format!("invalid upvalue index {}", index))?;
            *slot = src;
            Ok(())
        }
    }

    fn dispatch__mimium_global(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self._mimium_global();
        Vec::new()
    }

    #[inline(always)]
    fn _mimium_global(&mut self) -> () {
        let mut reg_264: Word = 0u64;
        return ();
    }

    fn dispatch_math_PI(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.math_PI();
        [result].to_vec()
    }

    #[inline(always)]
    fn math_PI(&mut self) -> Word {
        let mut reg_0: mmmfloat = 0.0 as mmmfloat;
        let mut reg_1: Word = 0u64;
        reg_0 = 3.14159265359 as mmmfloat;
        return f64_to_word(reg_0);
    }

    fn dispatch_math_E(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.math_E();
        [result].to_vec()
    }

    #[inline(always)]
    fn math_E(&mut self) -> Word {
        let mut reg_2: mmmfloat = 0.0 as mmmfloat;
        let mut reg_3: Word = 0u64;
        reg_2 = 2.71828182846 as mmmfloat;
        return f64_to_word(reg_2);
    }

    fn dispatch_math_exp(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.math_exp(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn math_exp(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let mut reg_4: Word = 0u64;
        let mut reg_5: mmmfloat = 0.0 as mmmfloat;
        let mut reg_6: mmmfloat = 0.0 as mmmfloat;
        let mut reg_7: mmmfloat = 0.0 as mmmfloat;
        let mut reg_8: Word = 0u64;
        reg_4 = 2u64;
        let call_result = self.math_E();
        reg_5 = word_to_f64(call_result);
        reg_6 = arg_0_scalar;
        reg_7 = reg_5.powf(reg_6);
        return f64_to_word(reg_7);
    }

    fn dispatch_math_log2(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.math_log2(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn math_log2(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let mut reg_9: mmmfloat = 0.0 as mmmfloat;
        let mut reg_10: mmmfloat = 0.0 as mmmfloat;
        let mut reg_11: mmmfloat = 0.0 as mmmfloat;
        let mut reg_12: mmmfloat = 0.0 as mmmfloat;
        let mut reg_13: mmmfloat = 0.0 as mmmfloat;
        let mut reg_14: Word = 0u64;
        reg_9 = arg_0_scalar;
        reg_10 = reg_9.ln();
        reg_11 = 2.0 as mmmfloat;
        reg_12 = reg_11.ln();
        reg_13 = reg_10 / reg_12;
        return f64_to_word(reg_13);
    }

    fn dispatch_math_log10(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.math_log10(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn math_log10(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let mut reg_15: mmmfloat = 0.0 as mmmfloat;
        let mut reg_16: mmmfloat = 0.0 as mmmfloat;
        let mut reg_17: mmmfloat = 0.0 as mmmfloat;
        let mut reg_18: mmmfloat = 0.0 as mmmfloat;
        let mut reg_19: mmmfloat = 0.0 as mmmfloat;
        let mut reg_20: Word = 0u64;
        reg_15 = arg_0_scalar;
        reg_16 = reg_15.ln();
        reg_17 = 10.0 as mmmfloat;
        reg_18 = reg_17.ln();
        reg_19 = reg_16 / reg_18;
        return f64_to_word(reg_19);
    }

    fn dispatch_osc_phasor_zero(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.osc_phasor_zero(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_phasor_zero(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let mut reg_21: mmmfloat = 0.0 as mmmfloat;
        let mut reg_22: mmmfloat = 0.0 as mmmfloat;
        let mut reg_23: mmmfloat = 0.0 as mmmfloat;
        let mut reg_24: mmmfloat = 0.0 as mmmfloat;
        let mut reg_25: mmmfloat = 0.0 as mmmfloat;
        let mut reg_26: mmmfloat = 0.0 as mmmfloat;
        let mut reg_27: mmmfloat = 0.0 as mmmfloat;
        let mut reg_28: mmmfloat = 0.0 as mmmfloat;
        let mut reg_29: Word = 0u64;
        reg_21 = word_to_f64({ let state = self.get_current_statestorage(); state.get_state_word() });
        reg_22 = reg_21;
        reg_23 = arg_0_scalar;
        reg_24 = self.host.sample_rate();
        reg_25 = reg_23 / reg_24;
        reg_26 = reg_22 + reg_25;
        reg_27 = 1.0 as mmmfloat;
        reg_28 = reg_26 % reg_27;
        {
            let state = self.get_current_statestorage();
            state.set_state_word(f64_to_word(reg_28));
        }
        let result = f64_to_word(reg_28);
        return result;
    }

    fn dispatch_osc_phasor(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_phasor(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_phasor(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_32: mmmfloat = 0.0 as mmmfloat;
        let mut reg_33: Word = 0u64;
        let mut reg_34: mmmfloat = 0.0 as mmmfloat;
        let mut reg_35: mmmfloat = 0.0 as mmmfloat;
        let mut reg_36: mmmfloat = 0.0 as mmmfloat;
        let mut reg_37: mmmfloat = 0.0 as mmmfloat;
        let mut reg_38: mmmfloat = 0.0 as mmmfloat;
        let mut reg_39: Word = 0u64;
        reg_32 = arg_0_scalar;
        reg_33 = 6u64;
        let call_result = self.osc_phasor_zero(f64_to_word(reg_32));
        reg_34 = word_to_f64(call_result);
        reg_35 = arg_1_scalar;
        reg_36 = reg_34 + reg_35;
        reg_37 = 1.0 as mmmfloat;
        reg_38 = reg_36 % reg_37;
        return f64_to_word(reg_38);
    }

    fn dispatch___default_7_phase_shift(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_7_phase_shift();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_7_phase_shift(&mut self) -> Word {
        let mut reg_30: mmmfloat = 0.0 as mmmfloat;
        let mut reg_31: Word = 0u64;
        reg_30 = 0.0 as mmmfloat;
        return f64_to_word(reg_30);
    }

    fn dispatch_osc_lfsaw(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_lfsaw(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_lfsaw(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_42: mmmfloat = 0.0 as mmmfloat;
        let mut reg_43: mmmfloat = 0.0 as mmmfloat;
        let mut reg_44: Word = 0u64;
        let mut reg_45: mmmfloat = 0.0 as mmmfloat;
        let mut reg_46: mmmfloat = 0.0 as mmmfloat;
        let mut reg_47: mmmfloat = 0.0 as mmmfloat;
        let mut reg_48: mmmfloat = 0.0 as mmmfloat;
        let mut reg_49: mmmfloat = 0.0 as mmmfloat;
        let mut reg_50: Word = 0u64;
        reg_42 = arg_0_scalar;
        reg_43 = arg_1_scalar;
        reg_44 = 7u64;
        let call_result = self.osc_phasor(f64_to_word(reg_42), f64_to_word(reg_43));
        reg_45 = word_to_f64(call_result);
        reg_46 = 2.0 as mmmfloat;
        reg_47 = reg_45 * reg_46;
        reg_48 = 1.0 as mmmfloat;
        reg_49 = reg_47 - reg_48;
        return f64_to_word(reg_49);
    }

    fn dispatch___default_9_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_9_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_9_phase(&mut self) -> Word {
        let mut reg_40: mmmfloat = 0.0 as mmmfloat;
        let mut reg_41: Word = 0u64;
        reg_40 = 0.0 as mmmfloat;
        return f64_to_word(reg_40);
    }

    fn dispatch_osc_saw(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_saw(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_saw(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_53: mmmfloat = 0.0 as mmmfloat;
        let mut reg_54: mmmfloat = 0.0 as mmmfloat;
        let mut reg_55: Word = 0u64;
        let mut reg_56: mmmfloat = 0.0 as mmmfloat;
        let mut reg_57: Word = 0u64;
        reg_53 = arg_0_scalar;
        reg_54 = arg_1_scalar;
        reg_55 = 9u64;
        let call_result = self.osc_lfsaw(f64_to_word(reg_53), f64_to_word(reg_54));
        reg_56 = word_to_f64(call_result);
        return f64_to_word(reg_56);
    }

    fn dispatch___default_11_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_11_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_11_phase(&mut self) -> Word {
        let mut reg_51: mmmfloat = 0.0 as mmmfloat;
        let mut reg_52: Word = 0u64;
        reg_51 = 0.0 as mmmfloat;
        return f64_to_word(reg_51);
    }

    fn dispatch_osc_tri(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_tri(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_tri(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_60: mmmfloat = 0.0 as mmmfloat;
        let mut reg_61: mmmfloat = 0.0 as mmmfloat;
        let mut reg_62: Word = 0u64;
        let mut reg_63: mmmfloat = 0.0 as mmmfloat;
        let mut reg_64: Word = 0u64;
        let mut reg_65: Word = 0u64;
        let mut reg_66: mmmfloat = 0.0 as mmmfloat;
        let mut reg_67: mmmfloat = 0.0 as mmmfloat;
        let mut reg_68: mmmfloat = 0.0 as mmmfloat;
        let mut reg_69: Word = 0u64;
        let mut reg_70: mmmfloat = 0.0 as mmmfloat;
        let mut reg_71: mmmfloat = 0.0 as mmmfloat;
        let mut reg_72: mmmfloat = 0.0 as mmmfloat;
        let mut reg_73: mmmfloat = 0.0 as mmmfloat;
        let mut reg_74: mmmfloat = 0.0 as mmmfloat;
        let mut reg_75: mmmfloat = 0.0 as mmmfloat;
        let mut reg_76: mmmfloat = 0.0 as mmmfloat;
        let mut reg_77: mmmfloat = 0.0 as mmmfloat;
        let mut reg_78: mmmfloat = 0.0 as mmmfloat;
        let mut reg_79: mmmfloat = 0.0 as mmmfloat;
        let mut reg_80: Word = 0u64;
        let mut reg_81: mmmfloat = 0.0 as mmmfloat;
        let mut reg_82: mmmfloat = 0.0 as mmmfloat;
        let mut reg_83: mmmfloat = 0.0 as mmmfloat;
        let mut reg_84: mmmfloat = 0.0 as mmmfloat;
        let mut reg_85: mmmfloat = 0.0 as mmmfloat;
        let mut reg_86: mmmfloat = 0.0 as mmmfloat;
        let mut reg_87: mmmfloat = 0.0 as mmmfloat;
        let mut reg_88: mmmfloat = 0.0 as mmmfloat;
        let mut reg_89: mmmfloat = 0.0 as mmmfloat;
        let mut reg_90: mmmfloat = 0.0 as mmmfloat;
        let mut reg_91: Word = 0u64;
        let mut reg_92: Word = 0u64;
        let mut reg_93: mmmfloat = 0.0 as mmmfloat;
        let mut reg_94: mmmfloat = 0.0 as mmmfloat;
        let mut reg_95: mmmfloat = 0.0 as mmmfloat;
        let mut reg_96: mmmfloat = 0.0 as mmmfloat;
        let mut reg_97: mmmfloat = 0.0 as mmmfloat;
        let mut reg_98: Word = 0u64;
        let mut stack_alloc_64 = [0u64; 1];
        let mut stack_alloc_91 = [0u64; 1];
        let mut bb: usize = 0;
        let mut pred_bb: usize = 0;
        loop {
            match bb {
                0 => {
                    reg_60 = arg_0_scalar;
                    reg_61 = arg_1_scalar;
                    reg_62 = 7u64;
                    let call_result = self.osc_phasor(f64_to_word(reg_60), f64_to_word(reg_61));
                    reg_63 = word_to_f64(call_result);
                    stack_alloc_64[0usize] = f64_to_word(reg_63);
                    reg_66 = word_to_f64(stack_alloc_64[0usize]);
                    reg_67 = 0.25 as mmmfloat;
                    reg_68 = if reg_66 < reg_67 { 1.0 } else { 0.0 };
                    pred_bb = 0;
                    bb = if truthy(f64_to_word(reg_68)) { 1usize } else { 2usize };
                    continue;
                },
                1 => {
                    reg_70 = word_to_f64(stack_alloc_64[0usize]);
                    reg_71 = 0.0 as mmmfloat;
                    reg_72 = 1.0 as mmmfloat;
                    reg_73 = reg_71 - reg_72;
                    reg_74 = reg_70 * reg_73;
                    reg_75 = 0.5 as mmmfloat;
                    reg_76 = reg_74 + reg_75;
                    pred_bb = 1;
                    bb = 6usize;
                    continue;
                },
                2 => {
                    reg_77 = word_to_f64(stack_alloc_64[0usize]);
                    reg_78 = 0.75 as mmmfloat;
                    reg_79 = if reg_77 > reg_78 { 1.0 } else { 0.0 };
                    pred_bb = 2;
                    bb = if truthy(f64_to_word(reg_79)) { 3usize } else { 4usize };
                    continue;
                    pred_bb = 2;
                    bb = 6usize;
                    continue;
                },
                3 => {
                    reg_81 = word_to_f64(stack_alloc_64[0usize]);
                    reg_82 = 0.0 as mmmfloat;
                    reg_83 = 1.0 as mmmfloat;
                    reg_84 = reg_82 - reg_83;
                    reg_85 = reg_81 * reg_84;
                    reg_86 = 1.5 as mmmfloat;
                    reg_87 = reg_85 + reg_86;
                    pred_bb = 3;
                    bb = 5usize;
                    continue;
                },
                4 => {
                    reg_88 = word_to_f64(stack_alloc_64[0usize]);
                    pred_bb = 4;
                    bb = 5usize;
                    continue;
                },
                5 => {
                    if pred_bb == 3usize {
                        reg_89 = reg_87;
                    } else if pred_bb == 4usize {
                        reg_89 = reg_88;
                    } else {
                        panic!("{}", format!("phi predecessor mismatch in block 5: {}", pred_bb));
                    }
                    pred_bb = 2;
                    bb = 6usize;
                    continue;
                },
                6 => {
                    if pred_bb == 1usize {
                        reg_90 = reg_76;
                    } else if pred_bb == 2usize {
                        reg_90 = reg_89;
                    } else {
                        panic!("{}", format!("phi predecessor mismatch in block 6: {}", pred_bb));
                    }
                    stack_alloc_91[0usize] = f64_to_word(reg_90);
                    reg_93 = word_to_f64(stack_alloc_91[0usize]);
                    reg_94 = 0.5 as mmmfloat;
                    reg_95 = reg_93 - reg_94;
                    reg_96 = 4.0 as mmmfloat;
                    reg_97 = reg_95 * reg_96;
                    return f64_to_word(reg_97);
                },
                _ => panic!("{}", format!("invalid basic block {} in function 13", bb)),
            }
        }
    }

    fn dispatch___default_13_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_13_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_13_phase(&mut self) -> Word {
        let mut reg_58: mmmfloat = 0.0 as mmmfloat;
        let mut reg_59: Word = 0u64;
        reg_58 = 0.0 as mmmfloat;
        return f64_to_word(reg_58);
    }

    fn dispatch_osc_lftri(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_lftri(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_lftri(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_101: mmmfloat = 0.0 as mmmfloat;
        let mut reg_102: mmmfloat = 0.0 as mmmfloat;
        let mut reg_103: Word = 0u64;
        let mut reg_104: mmmfloat = 0.0 as mmmfloat;
        let mut reg_105: mmmfloat = 0.0 as mmmfloat;
        let mut reg_106: mmmfloat = 0.0 as mmmfloat;
        let mut reg_107: mmmfloat = 0.0 as mmmfloat;
        let mut reg_108: mmmfloat = 0.0 as mmmfloat;
        let mut reg_109: Word = 0u64;
        reg_101 = arg_0_scalar;
        reg_102 = arg_1_scalar;
        reg_103 = 13u64;
        let call_result = self.osc_tri(f64_to_word(reg_101), f64_to_word(reg_102));
        reg_104 = word_to_f64(call_result);
        reg_105 = 0.5 as mmmfloat;
        reg_106 = reg_104 * reg_105;
        reg_107 = 0.5 as mmmfloat;
        reg_108 = reg_106 + reg_107;
        return f64_to_word(reg_108);
    }

    fn dispatch___default_15_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_15_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_15_phase(&mut self) -> Word {
        let mut reg_99: mmmfloat = 0.0 as mmmfloat;
        let mut reg_100: Word = 0u64;
        reg_99 = 0.0 as mmmfloat;
        return f64_to_word(reg_99);
    }

    fn dispatch_osc_rect(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let arg_2_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_2_value = arg_2_words[0];
        let result = self.osc_rect(arg_0_value, arg_1_value, arg_2_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_rect(&mut self, arg_0_value: Word, arg_1_value: Word, arg_2_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let arg_2 = [arg_2_value];
        let arg_2_scalar = word_to_f64(arg_2_value);
        let mut reg_114: mmmfloat = 0.0 as mmmfloat;
        let mut reg_115: mmmfloat = 0.0 as mmmfloat;
        let mut reg_116: Word = 0u64;
        let mut reg_117: mmmfloat = 0.0 as mmmfloat;
        let mut reg_118: mmmfloat = 0.0 as mmmfloat;
        let mut reg_119: mmmfloat = 0.0 as mmmfloat;
        let mut reg_120: Word = 0u64;
        let mut reg_121: mmmfloat = 0.0 as mmmfloat;
        let mut reg_122: mmmfloat = 0.0 as mmmfloat;
        let mut reg_123: mmmfloat = 0.0 as mmmfloat;
        let mut reg_124: mmmfloat = 0.0 as mmmfloat;
        let mut reg_125: mmmfloat = 0.0 as mmmfloat;
        let mut reg_126: Word = 0u64;
        let mut bb: usize = 0;
        let mut pred_bb: usize = 0;
        loop {
            match bb {
                0 => {
                    reg_114 = arg_0_scalar;
                    reg_115 = arg_1_scalar;
                    reg_116 = 7u64;
                    let call_result = self.osc_phasor(f64_to_word(reg_114), f64_to_word(reg_115));
                    reg_117 = word_to_f64(call_result);
                    reg_118 = arg_2_scalar;
                    reg_119 = if reg_117 < reg_118 { 1.0 } else { 0.0 };
                    pred_bb = 0;
                    bb = if truthy(f64_to_word(reg_119)) { 1usize } else { 2usize };
                    continue;
                },
                1 => {
                    reg_121 = 1.0 as mmmfloat;
                    pred_bb = 1;
                    bb = 3usize;
                    continue;
                },
                2 => {
                    reg_122 = 0.0 as mmmfloat;
                    reg_123 = 1.0 as mmmfloat;
                    reg_124 = reg_122 - reg_123;
                    pred_bb = 2;
                    bb = 3usize;
                    continue;
                },
                3 => {
                    if pred_bb == 1usize {
                        reg_125 = reg_121;
                    } else if pred_bb == 2usize {
                        reg_125 = reg_124;
                    } else {
                        panic!("{}", format!("phi predecessor mismatch in block 3: {}", pred_bb));
                    }
                    return f64_to_word(reg_125);
                },
                _ => panic!("{}", format!("invalid basic block {} in function 17", bb)),
            }
        }
    }

    fn dispatch___default_17_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_17_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_17_phase(&mut self) -> Word {
        let mut reg_110: mmmfloat = 0.0 as mmmfloat;
        let mut reg_111: Word = 0u64;
        reg_110 = 0.0 as mmmfloat;
        return f64_to_word(reg_110);
    }

    fn dispatch___default_17_duty(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_17_duty();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_17_duty(&mut self) -> Word {
        let mut reg_112: mmmfloat = 0.0 as mmmfloat;
        let mut reg_113: Word = 0u64;
        reg_112 = 0.5 as mmmfloat;
        return f64_to_word(reg_112);
    }

    fn dispatch_osc_lfrect(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let arg_2_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_2_value = arg_2_words[0];
        let result = self.osc_lfrect(arg_0_value, arg_1_value, arg_2_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_lfrect(&mut self, arg_0_value: Word, arg_1_value: Word, arg_2_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let arg_2 = [arg_2_value];
        let arg_2_scalar = word_to_f64(arg_2_value);
        let mut reg_131: mmmfloat = 0.0 as mmmfloat;
        let mut reg_132: mmmfloat = 0.0 as mmmfloat;
        let mut reg_133: Word = 0u64;
        let mut reg_134: mmmfloat = 0.0 as mmmfloat;
        let mut reg_135: mmmfloat = 0.0 as mmmfloat;
        let mut reg_136: mmmfloat = 0.0 as mmmfloat;
        let mut reg_137: Word = 0u64;
        let mut reg_138: mmmfloat = 0.0 as mmmfloat;
        let mut reg_139: mmmfloat = 0.0 as mmmfloat;
        let mut reg_140: mmmfloat = 0.0 as mmmfloat;
        let mut reg_141: Word = 0u64;
        let mut bb: usize = 0;
        let mut pred_bb: usize = 0;
        loop {
            match bb {
                0 => {
                    reg_131 = arg_0_scalar;
                    reg_132 = arg_1_scalar;
                    reg_133 = 7u64;
                    let call_result = self.osc_phasor(f64_to_word(reg_131), f64_to_word(reg_132));
                    reg_134 = word_to_f64(call_result);
                    reg_135 = arg_2_scalar;
                    reg_136 = if reg_134 < reg_135 { 1.0 } else { 0.0 };
                    pred_bb = 0;
                    bb = if truthy(f64_to_word(reg_136)) { 1usize } else { 2usize };
                    continue;
                },
                1 => {
                    reg_138 = 1.0 as mmmfloat;
                    pred_bb = 1;
                    bb = 3usize;
                    continue;
                },
                2 => {
                    reg_139 = 0.0 as mmmfloat;
                    pred_bb = 2;
                    bb = 3usize;
                    continue;
                },
                3 => {
                    if pred_bb == 1usize {
                        reg_140 = reg_138;
                    } else if pred_bb == 2usize {
                        reg_140 = reg_139;
                    } else {
                        panic!("{}", format!("phi predecessor mismatch in block 3: {}", pred_bb));
                    }
                    return f64_to_word(reg_140);
                },
                _ => panic!("{}", format!("invalid basic block {} in function 20", bb)),
            }
        }
    }

    fn dispatch___default_20_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_20_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_20_phase(&mut self) -> Word {
        let mut reg_127: mmmfloat = 0.0 as mmmfloat;
        let mut reg_128: Word = 0u64;
        reg_127 = 0.0 as mmmfloat;
        return f64_to_word(reg_127);
    }

    fn dispatch___default_20_duty(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_20_duty();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_20_duty(&mut self) -> Word {
        let mut reg_129: mmmfloat = 0.0 as mmmfloat;
        let mut reg_130: Word = 0u64;
        reg_129 = 0.5 as mmmfloat;
        return f64_to_word(reg_129);
    }

    fn dispatch_osc_sinwave(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_sinwave(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_sinwave(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_144: mmmfloat = 0.0 as mmmfloat;
        let mut reg_145: mmmfloat = 0.0 as mmmfloat;
        let mut reg_146: Word = 0u64;
        let mut reg_147: mmmfloat = 0.0 as mmmfloat;
        let mut reg_148: mmmfloat = 0.0 as mmmfloat;
        let mut reg_149: mmmfloat = 0.0 as mmmfloat;
        let mut reg_150: Word = 0u64;
        let mut reg_151: mmmfloat = 0.0 as mmmfloat;
        let mut reg_152: mmmfloat = 0.0 as mmmfloat;
        let mut reg_153: mmmfloat = 0.0 as mmmfloat;
        let mut reg_154: Word = 0u64;
        reg_144 = arg_0_scalar;
        reg_145 = arg_1_scalar;
        reg_146 = 7u64;
        let call_result = self.osc_phasor(f64_to_word(reg_144), f64_to_word(reg_145));
        reg_147 = word_to_f64(call_result);
        reg_148 = 2.0 as mmmfloat;
        reg_149 = reg_147 * reg_148;
        reg_150 = 1u64;
        let call_result = self.math_PI();
        reg_151 = word_to_f64(call_result);
        reg_152 = reg_149 * reg_151;
        reg_153 = reg_152.sin();
        return f64_to_word(reg_153);
    }

    fn dispatch___default_23_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_23_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_23_phase(&mut self) -> Word {
        let mut reg_142: mmmfloat = 0.0 as mmmfloat;
        let mut reg_143: Word = 0u64;
        reg_142 = 0.0 as mmmfloat;
        return f64_to_word(reg_142);
    }

    fn dispatch_osc_lfsinwave(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = arg_1_words[0];
        let result = self.osc_lfsinwave(arg_0_value, arg_1_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc_lfsinwave(&mut self, arg_0_value: Word, arg_1_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let arg_1 = [arg_1_value];
        let arg_1_scalar = word_to_f64(arg_1_value);
        let mut reg_157: mmmfloat = 0.0 as mmmfloat;
        let mut reg_158: mmmfloat = 0.0 as mmmfloat;
        let mut reg_159: Word = 0u64;
        let mut reg_160: mmmfloat = 0.0 as mmmfloat;
        let mut reg_161: mmmfloat = 0.0 as mmmfloat;
        let mut reg_162: mmmfloat = 0.0 as mmmfloat;
        let mut reg_163: mmmfloat = 0.0 as mmmfloat;
        let mut reg_164: mmmfloat = 0.0 as mmmfloat;
        let mut reg_165: Word = 0u64;
        reg_157 = arg_0_scalar;
        reg_158 = arg_1_scalar;
        reg_159 = 23u64;
        let call_result = self.osc_sinwave(f64_to_word(reg_157), f64_to_word(reg_158));
        reg_160 = word_to_f64(call_result);
        reg_161 = 0.5 as mmmfloat;
        reg_162 = reg_160 * reg_161;
        reg_163 = 0.5 as mmmfloat;
        reg_164 = reg_162 + reg_163;
        return f64_to_word(reg_164);
    }

    fn dispatch___default_25_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_25_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_25_phase(&mut self) -> Word {
        let mut reg_155: mmmfloat = 0.0 as mmmfloat;
        let mut reg_156: Word = 0u64;
        reg_155 = 0.0 as mmmfloat;
        return f64_to_word(reg_155);
    }

    fn dispatch_osc(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.osc(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn osc(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let arg_0_scalar = word_to_f64(arg_0_value);
        let mut reg_166: mmmfloat = 0.0 as mmmfloat;
        let mut reg_167: mmmfloat = 0.0 as mmmfloat;
        let mut reg_168: Word = 0u64;
        let mut reg_169: mmmfloat = 0.0 as mmmfloat;
        let mut reg_170: Word = 0u64;
        reg_166 = arg_0_scalar;
        reg_167 = 0.0 as mmmfloat;
        reg_168 = 23u64;
        let call_result = self.osc_sinwave(f64_to_word(reg_166), f64_to_word(reg_167));
        reg_169 = word_to_f64(call_result);
        return f64_to_word(reg_169);
    }

    fn dispatch_dsp(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let mut ret_words = [0u64; 2];
        self.dsp(&mut ret_words);
        ret_words.to_vec()
    }

    #[inline(always)]
    fn dsp(&mut self, ret_words: &mut [Word; 2]) -> () {
        let mut reg_171: mmmfloat = 0.0 as mmmfloat;
        let mut reg_172: mmmfloat = 0.0 as mmmfloat;
        let mut reg_173: mmmfloat = 0.0 as mmmfloat;
        let mut reg_174: Word = 0u64;
        let mut reg_175: mmmfloat = 0.0 as mmmfloat;
        let mut reg_176: mmmfloat = 0.0 as mmmfloat;
        let mut reg_177: mmmfloat = 0.0 as mmmfloat;
        let mut reg_178: mmmfloat = 0.0 as mmmfloat;
        let mut reg_179: mmmfloat = 0.0 as mmmfloat;
        let mut reg_180: mmmfloat = 0.0 as mmmfloat;
        let mut reg_181: Word = 0u64;
        let mut reg_182: mmmfloat = 0.0 as mmmfloat;
        let mut reg_183: mmmfloat = 0.0 as mmmfloat;
        let mut reg_184: mmmfloat = 0.0 as mmmfloat;
        let mut reg_185: mmmfloat = 0.0 as mmmfloat;
        let mut reg_186: mmmfloat = 0.0 as mmmfloat;
        let mut reg_187: mmmfloat = 0.0 as mmmfloat;
        let mut reg_188: Word = 0u64;
        let mut reg_189: mmmfloat = 0.0 as mmmfloat;
        let mut reg_190: mmmfloat = 0.0 as mmmfloat;
        let mut reg_191: mmmfloat = 0.0 as mmmfloat;
        let mut reg_192: mmmfloat = 0.0 as mmmfloat;
        let mut reg_193: mmmfloat = 0.0 as mmmfloat;
        let mut reg_194: mmmfloat = 0.0 as mmmfloat;
        let mut reg_195: Word = 0u64;
        let mut reg_196: mmmfloat = 0.0 as mmmfloat;
        let mut reg_197: mmmfloat = 0.0 as mmmfloat;
        let mut reg_198: mmmfloat = 0.0 as mmmfloat;
        let mut reg_199: mmmfloat = 0.0 as mmmfloat;
        let mut reg_200: mmmfloat = 0.0 as mmmfloat;
        let mut reg_201: mmmfloat = 0.0 as mmmfloat;
        let mut reg_202: Word = 0u64;
        let mut reg_203: mmmfloat = 0.0 as mmmfloat;
        let mut reg_204: mmmfloat = 0.0 as mmmfloat;
        let mut reg_205: mmmfloat = 0.0 as mmmfloat;
        let mut reg_206: mmmfloat = 0.0 as mmmfloat;
        let mut reg_207: mmmfloat = 0.0 as mmmfloat;
        let mut reg_208: mmmfloat = 0.0 as mmmfloat;
        let mut reg_209: Word = 0u64;
        let mut reg_210: mmmfloat = 0.0 as mmmfloat;
        let mut reg_211: mmmfloat = 0.0 as mmmfloat;
        let mut reg_212: mmmfloat = 0.0 as mmmfloat;
        let mut reg_213: mmmfloat = 0.0 as mmmfloat;
        let mut reg_214: mmmfloat = 0.0 as mmmfloat;
        let mut reg_215: mmmfloat = 0.0 as mmmfloat;
        let mut reg_216: Word = 0u64;
        let mut reg_217: mmmfloat = 0.0 as mmmfloat;
        let mut reg_218: mmmfloat = 0.0 as mmmfloat;
        let mut reg_219: mmmfloat = 0.0 as mmmfloat;
        let mut reg_220: mmmfloat = 0.0 as mmmfloat;
        let mut reg_221: mmmfloat = 0.0 as mmmfloat;
        let mut reg_222: mmmfloat = 0.0 as mmmfloat;
        let mut reg_223: Word = 0u64;
        let mut reg_224: mmmfloat = 0.0 as mmmfloat;
        let mut reg_225: mmmfloat = 0.0 as mmmfloat;
        let mut reg_226: mmmfloat = 0.0 as mmmfloat;
        let mut reg_227: mmmfloat = 0.0 as mmmfloat;
        let mut reg_228: mmmfloat = 0.0 as mmmfloat;
        let mut reg_229: mmmfloat = 0.0 as mmmfloat;
        let mut reg_230: Word = 0u64;
        let mut reg_231: mmmfloat = 0.0 as mmmfloat;
        let mut reg_232: mmmfloat = 0.0 as mmmfloat;
        let mut reg_233: mmmfloat = 0.0 as mmmfloat;
        let mut reg_234: mmmfloat = 0.0 as mmmfloat;
        let mut reg_235: mmmfloat = 0.0 as mmmfloat;
        let mut reg_236: mmmfloat = 0.0 as mmmfloat;
        let mut reg_237: Word = 0u64;
        let mut reg_238: mmmfloat = 0.0 as mmmfloat;
        let mut reg_239: mmmfloat = 0.0 as mmmfloat;
        let mut reg_240: mmmfloat = 0.0 as mmmfloat;
        let mut reg_241: mmmfloat = 0.0 as mmmfloat;
        let mut reg_242: Word = 0u64;
        let mut reg_243: mmmfloat = 0.0 as mmmfloat;
        let mut reg_244: mmmfloat = 0.0 as mmmfloat;
        let mut reg_245: mmmfloat = 0.0 as mmmfloat;
        let mut reg_246: mmmfloat = 0.0 as mmmfloat;
        let mut reg_247: mmmfloat = 0.0 as mmmfloat;
        let mut reg_248: mmmfloat = 0.0 as mmmfloat;
        let mut reg_249: mmmfloat = 0.0 as mmmfloat;
        let mut reg_250: mmmfloat = 0.0 as mmmfloat;
        let mut reg_251: mmmfloat = 0.0 as mmmfloat;
        let mut reg_252: mmmfloat = 0.0 as mmmfloat;
        let mut reg_253: mmmfloat = 0.0 as mmmfloat;
        let mut reg_254: Word = 0u64;
        let mut reg_255: Word = 0u64;
        let mut reg_256: Word = 0u64;
        let mut reg_257: mmmfloat = 0.0 as mmmfloat;
        let mut reg_258: Word = 0u64;
        let mut reg_259: Word = 0u64;
        let mut reg_260: mmmfloat = 0.0 as mmmfloat;
        let mut reg_261: Word = 0u64;
        let mut reg_262: Word = 0u64;
        let mut reg_263: Word = 0u64;
        let mut stack_alloc_254 = [0u64; 1];
        let mut stack_alloc_256 = [0u64; 2];
        reg_171 = 50.0 as mmmfloat;
        reg_172 = 10.0 as mmmfloat;
        reg_173 = reg_171 * reg_172;
        reg_174 = 27u64;
        let call_result = self.osc(f64_to_word(reg_173));
        reg_175 = word_to_f64(call_result);
        reg_176 = 10.0 as mmmfloat;
        reg_177 = reg_175 / reg_176;
        reg_178 = 50.0 as mmmfloat;
        reg_179 = 9.0 as mmmfloat;
        reg_180 = reg_178 * reg_179;
        self.get_current_statestorage().push_pos(1usize);
        reg_181 = 27u64;
        let call_result = self.osc(f64_to_word(reg_180));
        reg_182 = word_to_f64(call_result);
        reg_183 = 9.0 as mmmfloat;
        reg_184 = reg_182 / reg_183;
        reg_185 = 50.0 as mmmfloat;
        reg_186 = 8.0 as mmmfloat;
        reg_187 = reg_185 * reg_186;
        self.get_current_statestorage().push_pos(1usize);
        reg_188 = 27u64;
        let call_result = self.osc(f64_to_word(reg_187));
        reg_189 = word_to_f64(call_result);
        reg_190 = 8.0 as mmmfloat;
        reg_191 = reg_189 / reg_190;
        reg_192 = 50.0 as mmmfloat;
        reg_193 = 7.0 as mmmfloat;
        reg_194 = reg_192 * reg_193;
        self.get_current_statestorage().push_pos(1usize);
        reg_195 = 27u64;
        let call_result = self.osc(f64_to_word(reg_194));
        reg_196 = word_to_f64(call_result);
        reg_197 = 7.0 as mmmfloat;
        reg_198 = reg_196 / reg_197;
        reg_199 = 50.0 as mmmfloat;
        reg_200 = 6.0 as mmmfloat;
        reg_201 = reg_199 * reg_200;
        self.get_current_statestorage().push_pos(1usize);
        reg_202 = 27u64;
        let call_result = self.osc(f64_to_word(reg_201));
        reg_203 = word_to_f64(call_result);
        reg_204 = 6.0 as mmmfloat;
        reg_205 = reg_203 / reg_204;
        reg_206 = 50.0 as mmmfloat;
        reg_207 = 5.0 as mmmfloat;
        reg_208 = reg_206 * reg_207;
        self.get_current_statestorage().push_pos(1usize);
        reg_209 = 27u64;
        let call_result = self.osc(f64_to_word(reg_208));
        reg_210 = word_to_f64(call_result);
        reg_211 = 5.0 as mmmfloat;
        reg_212 = reg_210 / reg_211;
        reg_213 = 50.0 as mmmfloat;
        reg_214 = 4.0 as mmmfloat;
        reg_215 = reg_213 * reg_214;
        self.get_current_statestorage().push_pos(1usize);
        reg_216 = 27u64;
        let call_result = self.osc(f64_to_word(reg_215));
        reg_217 = word_to_f64(call_result);
        reg_218 = 4.0 as mmmfloat;
        reg_219 = reg_217 / reg_218;
        reg_220 = 50.0 as mmmfloat;
        reg_221 = 3.0 as mmmfloat;
        reg_222 = reg_220 * reg_221;
        self.get_current_statestorage().push_pos(1usize);
        reg_223 = 27u64;
        let call_result = self.osc(f64_to_word(reg_222));
        reg_224 = word_to_f64(call_result);
        reg_225 = 3.0 as mmmfloat;
        reg_226 = reg_224 / reg_225;
        reg_227 = 50.0 as mmmfloat;
        reg_228 = 2.0 as mmmfloat;
        reg_229 = reg_227 * reg_228;
        self.get_current_statestorage().push_pos(1usize);
        reg_230 = 27u64;
        let call_result = self.osc(f64_to_word(reg_229));
        reg_231 = word_to_f64(call_result);
        reg_232 = 2.0 as mmmfloat;
        reg_233 = reg_231 / reg_232;
        reg_234 = 50.0 as mmmfloat;
        reg_235 = 1.0 as mmmfloat;
        reg_236 = reg_234 * reg_235;
        self.get_current_statestorage().push_pos(1usize);
        reg_237 = 27u64;
        let call_result = self.osc(f64_to_word(reg_236));
        reg_238 = word_to_f64(call_result);
        reg_239 = 1.0 as mmmfloat;
        reg_240 = reg_238 / reg_239;
        reg_241 = 50.0 as mmmfloat;
        self.get_current_statestorage().push_pos(1usize);
        reg_242 = 27u64;
        let call_result = self.osc(f64_to_word(reg_241));
        reg_243 = word_to_f64(call_result);
        reg_244 = reg_240 + reg_243;
        reg_245 = reg_233 + reg_244;
        reg_246 = reg_226 + reg_245;
        reg_247 = reg_219 + reg_246;
        reg_248 = reg_212 + reg_247;
        reg_249 = reg_205 + reg_248;
        reg_250 = reg_198 + reg_249;
        reg_251 = reg_191 + reg_250;
        reg_252 = reg_184 + reg_251;
        reg_253 = reg_177 + reg_252;
        stack_alloc_254[0usize] = f64_to_word(reg_253);
        reg_257 = word_to_f64(stack_alloc_254[0usize]);
        stack_alloc_256[0usize] = f64_to_word(reg_257);
        reg_260 = word_to_f64(stack_alloc_254[0usize]);
        stack_alloc_256[1usize] = f64_to_word(reg_260);
        self.get_current_statestorage().pop_pos(10usize);
        ret_words.copy_from_slice(&stack_alloc_256[0usize..2usize]);
        return ();
    }

}
