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
                StateStorage::new(101),

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
        let mut reg_984: Word = 0u64;
        return ();
    }

    fn dispatch_math_PI(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.math_PI();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn math_PI(&mut self) -> mmmfloat {
        let mut reg_0: mmmfloat = 0.0 as mmmfloat;
        let mut reg_1: Word = 0u64;
        reg_0 = 3.14159265359 as mmmfloat;
        return reg_0;
    }

    fn dispatch_math_E(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.math_E();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn math_E(&mut self) -> mmmfloat {
        let mut reg_2: mmmfloat = 0.0 as mmmfloat;
        let mut reg_3: Word = 0u64;
        reg_2 = 2.71828182846 as mmmfloat;
        return reg_2;
    }

    fn dispatch_math_exp(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let result = self.math_exp(arg_0_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn math_exp(&mut self, arg_0_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let mut reg_4: Word = 0u64;
        let mut reg_5: mmmfloat = 0.0 as mmmfloat;
        let mut reg_6: mmmfloat = 0.0 as mmmfloat;
        let mut reg_7: mmmfloat = 0.0 as mmmfloat;
        let mut reg_8: Word = 0u64;
        reg_4 = 2u64;
        let call_result = self.math_E();
        reg_5 = call_result;
        reg_6 = arg_0_scalar;
        reg_7 = reg_5.powf(reg_6);
        return reg_7;
    }

    fn dispatch_math_log2(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let result = self.math_log2(arg_0_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn math_log2(&mut self, arg_0_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
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
        return reg_13;
    }

    fn dispatch_math_log10(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let result = self.math_log10(arg_0_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn math_log10(&mut self, arg_0_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
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
        return reg_19;
    }

    fn dispatch_osc_phasor_zero(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let result = self.osc_phasor_zero(arg_0_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_phasor_zero(&mut self, arg_0_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
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
        let result = reg_28;
        return result;
    }

    fn dispatch_osc_phasor(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_phasor(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_phasor(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
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
        let call_result = self.osc_phasor_zero(reg_32);
        reg_34 = call_result;
        reg_35 = arg_1_scalar;
        reg_36 = reg_34 + reg_35;
        reg_37 = 1.0 as mmmfloat;
        reg_38 = reg_36 % reg_37;
        return reg_38;
    }

    fn dispatch___default_7_phase_shift(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_7_phase_shift();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_7_phase_shift(&mut self) -> mmmfloat {
        let mut reg_30: mmmfloat = 0.0 as mmmfloat;
        let mut reg_31: Word = 0u64;
        reg_30 = 0.0 as mmmfloat;
        return reg_30;
    }

    fn dispatch_osc_lfsaw(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_lfsaw(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_lfsaw(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
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
        let call_result = self.osc_phasor(reg_42, reg_43);
        reg_45 = call_result;
        reg_46 = 2.0 as mmmfloat;
        reg_47 = reg_45 * reg_46;
        reg_48 = 1.0 as mmmfloat;
        reg_49 = reg_47 - reg_48;
        return reg_49;
    }

    fn dispatch___default_9_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_9_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_9_phase(&mut self) -> mmmfloat {
        let mut reg_40: mmmfloat = 0.0 as mmmfloat;
        let mut reg_41: Word = 0u64;
        reg_40 = 0.0 as mmmfloat;
        return reg_40;
    }

    fn dispatch_osc_saw(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_saw(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_saw(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
        let mut reg_53: mmmfloat = 0.0 as mmmfloat;
        let mut reg_54: mmmfloat = 0.0 as mmmfloat;
        let mut reg_55: Word = 0u64;
        let mut reg_56: mmmfloat = 0.0 as mmmfloat;
        let mut reg_57: Word = 0u64;
        reg_53 = arg_0_scalar;
        reg_54 = arg_1_scalar;
        reg_55 = 9u64;
        let call_result = self.osc_lfsaw(reg_53, reg_54);
        reg_56 = call_result;
        return reg_56;
    }

    fn dispatch___default_11_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_11_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_11_phase(&mut self) -> mmmfloat {
        let mut reg_51: mmmfloat = 0.0 as mmmfloat;
        let mut reg_52: Word = 0u64;
        reg_51 = 0.0 as mmmfloat;
        return reg_51;
    }

    fn dispatch_osc_tri(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_tri(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_tri(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
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
                    let call_result = self.osc_phasor(reg_60, reg_61);
                    reg_63 = call_result;
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
                    return reg_97;
                },
                _ => panic!("{}", format!("invalid basic block {} in function 13", bb)),
            }
        }
    }

    fn dispatch___default_13_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_13_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_13_phase(&mut self) -> mmmfloat {
        let mut reg_58: mmmfloat = 0.0 as mmmfloat;
        let mut reg_59: Word = 0u64;
        reg_58 = 0.0 as mmmfloat;
        return reg_58;
    }

    fn dispatch_osc_lftri(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_lftri(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_lftri(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
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
        let call_result = self.osc_tri(reg_101, reg_102);
        reg_104 = call_result;
        reg_105 = 0.5 as mmmfloat;
        reg_106 = reg_104 * reg_105;
        reg_107 = 0.5 as mmmfloat;
        reg_108 = reg_106 + reg_107;
        return reg_108;
    }

    fn dispatch___default_15_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_15_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_15_phase(&mut self) -> mmmfloat {
        let mut reg_99: mmmfloat = 0.0 as mmmfloat;
        let mut reg_100: Word = 0u64;
        reg_99 = 0.0 as mmmfloat;
        return reg_99;
    }

    fn dispatch_osc_rect(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let arg_2_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_2_value = word_to_f64(arg_2_words[0]);
        let result = self.osc_rect(arg_0_value, arg_1_value, arg_2_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_rect(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat, arg_2_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
        let arg_2 = [f64_to_word(arg_2_value)];
        let arg_2_scalar = arg_2_value;
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
                    let call_result = self.osc_phasor(reg_114, reg_115);
                    reg_117 = call_result;
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
                    return reg_125;
                },
                _ => panic!("{}", format!("invalid basic block {} in function 17", bb)),
            }
        }
    }

    fn dispatch___default_17_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_17_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_17_phase(&mut self) -> mmmfloat {
        let mut reg_110: mmmfloat = 0.0 as mmmfloat;
        let mut reg_111: Word = 0u64;
        reg_110 = 0.0 as mmmfloat;
        return reg_110;
    }

    fn dispatch___default_17_duty(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_17_duty();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_17_duty(&mut self) -> mmmfloat {
        let mut reg_112: mmmfloat = 0.0 as mmmfloat;
        let mut reg_113: Word = 0u64;
        reg_112 = 0.5 as mmmfloat;
        return reg_112;
    }

    fn dispatch_osc_lfrect(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let arg_2_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_2_value = word_to_f64(arg_2_words[0]);
        let result = self.osc_lfrect(arg_0_value, arg_1_value, arg_2_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_lfrect(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat, arg_2_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
        let arg_2 = [f64_to_word(arg_2_value)];
        let arg_2_scalar = arg_2_value;
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
                    let call_result = self.osc_phasor(reg_131, reg_132);
                    reg_134 = call_result;
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
                    return reg_140;
                },
                _ => panic!("{}", format!("invalid basic block {} in function 20", bb)),
            }
        }
    }

    fn dispatch___default_20_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_20_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_20_phase(&mut self) -> mmmfloat {
        let mut reg_127: mmmfloat = 0.0 as mmmfloat;
        let mut reg_128: Word = 0u64;
        reg_127 = 0.0 as mmmfloat;
        return reg_127;
    }

    fn dispatch___default_20_duty(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_20_duty();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_20_duty(&mut self) -> mmmfloat {
        let mut reg_129: mmmfloat = 0.0 as mmmfloat;
        let mut reg_130: Word = 0u64;
        reg_129 = 0.5 as mmmfloat;
        return reg_129;
    }

    fn dispatch_osc_sinwave(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_sinwave(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_sinwave(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
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
        let call_result = self.osc_phasor(reg_144, reg_145);
        reg_147 = call_result;
        reg_148 = 2.0 as mmmfloat;
        reg_149 = reg_147 * reg_148;
        reg_150 = 1u64;
        let call_result = self.math_PI();
        reg_151 = call_result;
        reg_152 = reg_149 * reg_151;
        reg_153 = reg_152.sin();
        return reg_153;
    }

    fn dispatch___default_23_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_23_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_23_phase(&mut self) -> mmmfloat {
        let mut reg_142: mmmfloat = 0.0 as mmmfloat;
        let mut reg_143: Word = 0u64;
        reg_142 = 0.0 as mmmfloat;
        return reg_142;
    }

    fn dispatch_osc_lfsinwave(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let arg_1_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_1_value = word_to_f64(arg_1_words[0]);
        let result = self.osc_lfsinwave(arg_0_value, arg_1_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc_lfsinwave(&mut self, arg_0_value: mmmfloat, arg_1_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let arg_1 = [f64_to_word(arg_1_value)];
        let arg_1_scalar = arg_1_value;
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
        let call_result = self.osc_sinwave(reg_157, reg_158);
        reg_160 = call_result;
        reg_161 = 0.5 as mmmfloat;
        reg_162 = reg_160 * reg_161;
        reg_163 = 0.5 as mmmfloat;
        reg_164 = reg_162 + reg_163;
        return reg_164;
    }

    fn dispatch___default_25_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_25_phase();
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn __default_25_phase(&mut self) -> mmmfloat {
        let mut reg_155: mmmfloat = 0.0 as mmmfloat;
        let mut reg_156: Word = 0u64;
        reg_155 = 0.0 as mmmfloat;
        return reg_155;
    }

    fn dispatch_osc(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = word_to_f64(arg_0_words[0]);
        let result = self.osc(arg_0_value);
        [f64_to_word(result)].to_vec()
    }

    #[inline(always)]
    fn osc(&mut self, arg_0_value: mmmfloat) -> mmmfloat {
        let arg_0 = [f64_to_word(arg_0_value)];
        let arg_0_scalar = arg_0_value;
        let mut reg_166: mmmfloat = 0.0 as mmmfloat;
        let mut reg_167: mmmfloat = 0.0 as mmmfloat;
        let mut reg_168: Word = 0u64;
        let mut reg_169: mmmfloat = 0.0 as mmmfloat;
        let mut reg_170: Word = 0u64;
        reg_166 = arg_0_scalar;
        reg_167 = 0.0 as mmmfloat;
        reg_168 = 23u64;
        let call_result = self.osc_sinwave(reg_166, reg_167);
        reg_169 = call_result;
        return reg_169;
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
        let mut reg_242: mmmfloat = 0.0 as mmmfloat;
        let mut reg_243: mmmfloat = 0.0 as mmmfloat;
        let mut reg_244: Word = 0u64;
        let mut reg_245: mmmfloat = 0.0 as mmmfloat;
        let mut reg_246: mmmfloat = 0.0 as mmmfloat;
        let mut reg_247: mmmfloat = 0.0 as mmmfloat;
        let mut reg_248: mmmfloat = 0.0 as mmmfloat;
        let mut reg_249: mmmfloat = 0.0 as mmmfloat;
        let mut reg_250: mmmfloat = 0.0 as mmmfloat;
        let mut reg_251: Word = 0u64;
        let mut reg_252: mmmfloat = 0.0 as mmmfloat;
        let mut reg_253: mmmfloat = 0.0 as mmmfloat;
        let mut reg_254: mmmfloat = 0.0 as mmmfloat;
        let mut reg_255: mmmfloat = 0.0 as mmmfloat;
        let mut reg_256: mmmfloat = 0.0 as mmmfloat;
        let mut reg_257: mmmfloat = 0.0 as mmmfloat;
        let mut reg_258: Word = 0u64;
        let mut reg_259: mmmfloat = 0.0 as mmmfloat;
        let mut reg_260: mmmfloat = 0.0 as mmmfloat;
        let mut reg_261: mmmfloat = 0.0 as mmmfloat;
        let mut reg_262: mmmfloat = 0.0 as mmmfloat;
        let mut reg_263: mmmfloat = 0.0 as mmmfloat;
        let mut reg_264: mmmfloat = 0.0 as mmmfloat;
        let mut reg_265: Word = 0u64;
        let mut reg_266: mmmfloat = 0.0 as mmmfloat;
        let mut reg_267: mmmfloat = 0.0 as mmmfloat;
        let mut reg_268: mmmfloat = 0.0 as mmmfloat;
        let mut reg_269: mmmfloat = 0.0 as mmmfloat;
        let mut reg_270: mmmfloat = 0.0 as mmmfloat;
        let mut reg_271: mmmfloat = 0.0 as mmmfloat;
        let mut reg_272: Word = 0u64;
        let mut reg_273: mmmfloat = 0.0 as mmmfloat;
        let mut reg_274: mmmfloat = 0.0 as mmmfloat;
        let mut reg_275: mmmfloat = 0.0 as mmmfloat;
        let mut reg_276: mmmfloat = 0.0 as mmmfloat;
        let mut reg_277: mmmfloat = 0.0 as mmmfloat;
        let mut reg_278: mmmfloat = 0.0 as mmmfloat;
        let mut reg_279: Word = 0u64;
        let mut reg_280: mmmfloat = 0.0 as mmmfloat;
        let mut reg_281: mmmfloat = 0.0 as mmmfloat;
        let mut reg_282: mmmfloat = 0.0 as mmmfloat;
        let mut reg_283: mmmfloat = 0.0 as mmmfloat;
        let mut reg_284: mmmfloat = 0.0 as mmmfloat;
        let mut reg_285: mmmfloat = 0.0 as mmmfloat;
        let mut reg_286: Word = 0u64;
        let mut reg_287: mmmfloat = 0.0 as mmmfloat;
        let mut reg_288: mmmfloat = 0.0 as mmmfloat;
        let mut reg_289: mmmfloat = 0.0 as mmmfloat;
        let mut reg_290: mmmfloat = 0.0 as mmmfloat;
        let mut reg_291: mmmfloat = 0.0 as mmmfloat;
        let mut reg_292: mmmfloat = 0.0 as mmmfloat;
        let mut reg_293: Word = 0u64;
        let mut reg_294: mmmfloat = 0.0 as mmmfloat;
        let mut reg_295: mmmfloat = 0.0 as mmmfloat;
        let mut reg_296: mmmfloat = 0.0 as mmmfloat;
        let mut reg_297: mmmfloat = 0.0 as mmmfloat;
        let mut reg_298: mmmfloat = 0.0 as mmmfloat;
        let mut reg_299: mmmfloat = 0.0 as mmmfloat;
        let mut reg_300: Word = 0u64;
        let mut reg_301: mmmfloat = 0.0 as mmmfloat;
        let mut reg_302: mmmfloat = 0.0 as mmmfloat;
        let mut reg_303: mmmfloat = 0.0 as mmmfloat;
        let mut reg_304: mmmfloat = 0.0 as mmmfloat;
        let mut reg_305: mmmfloat = 0.0 as mmmfloat;
        let mut reg_306: mmmfloat = 0.0 as mmmfloat;
        let mut reg_307: Word = 0u64;
        let mut reg_308: mmmfloat = 0.0 as mmmfloat;
        let mut reg_309: mmmfloat = 0.0 as mmmfloat;
        let mut reg_310: mmmfloat = 0.0 as mmmfloat;
        let mut reg_311: mmmfloat = 0.0 as mmmfloat;
        let mut reg_312: mmmfloat = 0.0 as mmmfloat;
        let mut reg_313: mmmfloat = 0.0 as mmmfloat;
        let mut reg_314: Word = 0u64;
        let mut reg_315: mmmfloat = 0.0 as mmmfloat;
        let mut reg_316: mmmfloat = 0.0 as mmmfloat;
        let mut reg_317: mmmfloat = 0.0 as mmmfloat;
        let mut reg_318: mmmfloat = 0.0 as mmmfloat;
        let mut reg_319: mmmfloat = 0.0 as mmmfloat;
        let mut reg_320: mmmfloat = 0.0 as mmmfloat;
        let mut reg_321: Word = 0u64;
        let mut reg_322: mmmfloat = 0.0 as mmmfloat;
        let mut reg_323: mmmfloat = 0.0 as mmmfloat;
        let mut reg_324: mmmfloat = 0.0 as mmmfloat;
        let mut reg_325: mmmfloat = 0.0 as mmmfloat;
        let mut reg_326: mmmfloat = 0.0 as mmmfloat;
        let mut reg_327: mmmfloat = 0.0 as mmmfloat;
        let mut reg_328: Word = 0u64;
        let mut reg_329: mmmfloat = 0.0 as mmmfloat;
        let mut reg_330: mmmfloat = 0.0 as mmmfloat;
        let mut reg_331: mmmfloat = 0.0 as mmmfloat;
        let mut reg_332: mmmfloat = 0.0 as mmmfloat;
        let mut reg_333: mmmfloat = 0.0 as mmmfloat;
        let mut reg_334: mmmfloat = 0.0 as mmmfloat;
        let mut reg_335: Word = 0u64;
        let mut reg_336: mmmfloat = 0.0 as mmmfloat;
        let mut reg_337: mmmfloat = 0.0 as mmmfloat;
        let mut reg_338: mmmfloat = 0.0 as mmmfloat;
        let mut reg_339: mmmfloat = 0.0 as mmmfloat;
        let mut reg_340: mmmfloat = 0.0 as mmmfloat;
        let mut reg_341: mmmfloat = 0.0 as mmmfloat;
        let mut reg_342: Word = 0u64;
        let mut reg_343: mmmfloat = 0.0 as mmmfloat;
        let mut reg_344: mmmfloat = 0.0 as mmmfloat;
        let mut reg_345: mmmfloat = 0.0 as mmmfloat;
        let mut reg_346: mmmfloat = 0.0 as mmmfloat;
        let mut reg_347: mmmfloat = 0.0 as mmmfloat;
        let mut reg_348: mmmfloat = 0.0 as mmmfloat;
        let mut reg_349: Word = 0u64;
        let mut reg_350: mmmfloat = 0.0 as mmmfloat;
        let mut reg_351: mmmfloat = 0.0 as mmmfloat;
        let mut reg_352: mmmfloat = 0.0 as mmmfloat;
        let mut reg_353: mmmfloat = 0.0 as mmmfloat;
        let mut reg_354: mmmfloat = 0.0 as mmmfloat;
        let mut reg_355: mmmfloat = 0.0 as mmmfloat;
        let mut reg_356: Word = 0u64;
        let mut reg_357: mmmfloat = 0.0 as mmmfloat;
        let mut reg_358: mmmfloat = 0.0 as mmmfloat;
        let mut reg_359: mmmfloat = 0.0 as mmmfloat;
        let mut reg_360: mmmfloat = 0.0 as mmmfloat;
        let mut reg_361: mmmfloat = 0.0 as mmmfloat;
        let mut reg_362: mmmfloat = 0.0 as mmmfloat;
        let mut reg_363: Word = 0u64;
        let mut reg_364: mmmfloat = 0.0 as mmmfloat;
        let mut reg_365: mmmfloat = 0.0 as mmmfloat;
        let mut reg_366: mmmfloat = 0.0 as mmmfloat;
        let mut reg_367: mmmfloat = 0.0 as mmmfloat;
        let mut reg_368: mmmfloat = 0.0 as mmmfloat;
        let mut reg_369: mmmfloat = 0.0 as mmmfloat;
        let mut reg_370: Word = 0u64;
        let mut reg_371: mmmfloat = 0.0 as mmmfloat;
        let mut reg_372: mmmfloat = 0.0 as mmmfloat;
        let mut reg_373: mmmfloat = 0.0 as mmmfloat;
        let mut reg_374: mmmfloat = 0.0 as mmmfloat;
        let mut reg_375: mmmfloat = 0.0 as mmmfloat;
        let mut reg_376: mmmfloat = 0.0 as mmmfloat;
        let mut reg_377: Word = 0u64;
        let mut reg_378: mmmfloat = 0.0 as mmmfloat;
        let mut reg_379: mmmfloat = 0.0 as mmmfloat;
        let mut reg_380: mmmfloat = 0.0 as mmmfloat;
        let mut reg_381: mmmfloat = 0.0 as mmmfloat;
        let mut reg_382: mmmfloat = 0.0 as mmmfloat;
        let mut reg_383: mmmfloat = 0.0 as mmmfloat;
        let mut reg_384: Word = 0u64;
        let mut reg_385: mmmfloat = 0.0 as mmmfloat;
        let mut reg_386: mmmfloat = 0.0 as mmmfloat;
        let mut reg_387: mmmfloat = 0.0 as mmmfloat;
        let mut reg_388: mmmfloat = 0.0 as mmmfloat;
        let mut reg_389: mmmfloat = 0.0 as mmmfloat;
        let mut reg_390: mmmfloat = 0.0 as mmmfloat;
        let mut reg_391: Word = 0u64;
        let mut reg_392: mmmfloat = 0.0 as mmmfloat;
        let mut reg_393: mmmfloat = 0.0 as mmmfloat;
        let mut reg_394: mmmfloat = 0.0 as mmmfloat;
        let mut reg_395: mmmfloat = 0.0 as mmmfloat;
        let mut reg_396: mmmfloat = 0.0 as mmmfloat;
        let mut reg_397: mmmfloat = 0.0 as mmmfloat;
        let mut reg_398: Word = 0u64;
        let mut reg_399: mmmfloat = 0.0 as mmmfloat;
        let mut reg_400: mmmfloat = 0.0 as mmmfloat;
        let mut reg_401: mmmfloat = 0.0 as mmmfloat;
        let mut reg_402: mmmfloat = 0.0 as mmmfloat;
        let mut reg_403: mmmfloat = 0.0 as mmmfloat;
        let mut reg_404: mmmfloat = 0.0 as mmmfloat;
        let mut reg_405: Word = 0u64;
        let mut reg_406: mmmfloat = 0.0 as mmmfloat;
        let mut reg_407: mmmfloat = 0.0 as mmmfloat;
        let mut reg_408: mmmfloat = 0.0 as mmmfloat;
        let mut reg_409: mmmfloat = 0.0 as mmmfloat;
        let mut reg_410: mmmfloat = 0.0 as mmmfloat;
        let mut reg_411: mmmfloat = 0.0 as mmmfloat;
        let mut reg_412: Word = 0u64;
        let mut reg_413: mmmfloat = 0.0 as mmmfloat;
        let mut reg_414: mmmfloat = 0.0 as mmmfloat;
        let mut reg_415: mmmfloat = 0.0 as mmmfloat;
        let mut reg_416: mmmfloat = 0.0 as mmmfloat;
        let mut reg_417: mmmfloat = 0.0 as mmmfloat;
        let mut reg_418: mmmfloat = 0.0 as mmmfloat;
        let mut reg_419: Word = 0u64;
        let mut reg_420: mmmfloat = 0.0 as mmmfloat;
        let mut reg_421: mmmfloat = 0.0 as mmmfloat;
        let mut reg_422: mmmfloat = 0.0 as mmmfloat;
        let mut reg_423: mmmfloat = 0.0 as mmmfloat;
        let mut reg_424: mmmfloat = 0.0 as mmmfloat;
        let mut reg_425: mmmfloat = 0.0 as mmmfloat;
        let mut reg_426: Word = 0u64;
        let mut reg_427: mmmfloat = 0.0 as mmmfloat;
        let mut reg_428: mmmfloat = 0.0 as mmmfloat;
        let mut reg_429: mmmfloat = 0.0 as mmmfloat;
        let mut reg_430: mmmfloat = 0.0 as mmmfloat;
        let mut reg_431: mmmfloat = 0.0 as mmmfloat;
        let mut reg_432: mmmfloat = 0.0 as mmmfloat;
        let mut reg_433: Word = 0u64;
        let mut reg_434: mmmfloat = 0.0 as mmmfloat;
        let mut reg_435: mmmfloat = 0.0 as mmmfloat;
        let mut reg_436: mmmfloat = 0.0 as mmmfloat;
        let mut reg_437: mmmfloat = 0.0 as mmmfloat;
        let mut reg_438: mmmfloat = 0.0 as mmmfloat;
        let mut reg_439: mmmfloat = 0.0 as mmmfloat;
        let mut reg_440: Word = 0u64;
        let mut reg_441: mmmfloat = 0.0 as mmmfloat;
        let mut reg_442: mmmfloat = 0.0 as mmmfloat;
        let mut reg_443: mmmfloat = 0.0 as mmmfloat;
        let mut reg_444: mmmfloat = 0.0 as mmmfloat;
        let mut reg_445: mmmfloat = 0.0 as mmmfloat;
        let mut reg_446: mmmfloat = 0.0 as mmmfloat;
        let mut reg_447: Word = 0u64;
        let mut reg_448: mmmfloat = 0.0 as mmmfloat;
        let mut reg_449: mmmfloat = 0.0 as mmmfloat;
        let mut reg_450: mmmfloat = 0.0 as mmmfloat;
        let mut reg_451: mmmfloat = 0.0 as mmmfloat;
        let mut reg_452: mmmfloat = 0.0 as mmmfloat;
        let mut reg_453: mmmfloat = 0.0 as mmmfloat;
        let mut reg_454: Word = 0u64;
        let mut reg_455: mmmfloat = 0.0 as mmmfloat;
        let mut reg_456: mmmfloat = 0.0 as mmmfloat;
        let mut reg_457: mmmfloat = 0.0 as mmmfloat;
        let mut reg_458: mmmfloat = 0.0 as mmmfloat;
        let mut reg_459: mmmfloat = 0.0 as mmmfloat;
        let mut reg_460: mmmfloat = 0.0 as mmmfloat;
        let mut reg_461: Word = 0u64;
        let mut reg_462: mmmfloat = 0.0 as mmmfloat;
        let mut reg_463: mmmfloat = 0.0 as mmmfloat;
        let mut reg_464: mmmfloat = 0.0 as mmmfloat;
        let mut reg_465: mmmfloat = 0.0 as mmmfloat;
        let mut reg_466: mmmfloat = 0.0 as mmmfloat;
        let mut reg_467: mmmfloat = 0.0 as mmmfloat;
        let mut reg_468: Word = 0u64;
        let mut reg_469: mmmfloat = 0.0 as mmmfloat;
        let mut reg_470: mmmfloat = 0.0 as mmmfloat;
        let mut reg_471: mmmfloat = 0.0 as mmmfloat;
        let mut reg_472: mmmfloat = 0.0 as mmmfloat;
        let mut reg_473: mmmfloat = 0.0 as mmmfloat;
        let mut reg_474: mmmfloat = 0.0 as mmmfloat;
        let mut reg_475: Word = 0u64;
        let mut reg_476: mmmfloat = 0.0 as mmmfloat;
        let mut reg_477: mmmfloat = 0.0 as mmmfloat;
        let mut reg_478: mmmfloat = 0.0 as mmmfloat;
        let mut reg_479: mmmfloat = 0.0 as mmmfloat;
        let mut reg_480: mmmfloat = 0.0 as mmmfloat;
        let mut reg_481: mmmfloat = 0.0 as mmmfloat;
        let mut reg_482: Word = 0u64;
        let mut reg_483: mmmfloat = 0.0 as mmmfloat;
        let mut reg_484: mmmfloat = 0.0 as mmmfloat;
        let mut reg_485: mmmfloat = 0.0 as mmmfloat;
        let mut reg_486: mmmfloat = 0.0 as mmmfloat;
        let mut reg_487: mmmfloat = 0.0 as mmmfloat;
        let mut reg_488: mmmfloat = 0.0 as mmmfloat;
        let mut reg_489: Word = 0u64;
        let mut reg_490: mmmfloat = 0.0 as mmmfloat;
        let mut reg_491: mmmfloat = 0.0 as mmmfloat;
        let mut reg_492: mmmfloat = 0.0 as mmmfloat;
        let mut reg_493: mmmfloat = 0.0 as mmmfloat;
        let mut reg_494: mmmfloat = 0.0 as mmmfloat;
        let mut reg_495: mmmfloat = 0.0 as mmmfloat;
        let mut reg_496: Word = 0u64;
        let mut reg_497: mmmfloat = 0.0 as mmmfloat;
        let mut reg_498: mmmfloat = 0.0 as mmmfloat;
        let mut reg_499: mmmfloat = 0.0 as mmmfloat;
        let mut reg_500: mmmfloat = 0.0 as mmmfloat;
        let mut reg_501: mmmfloat = 0.0 as mmmfloat;
        let mut reg_502: mmmfloat = 0.0 as mmmfloat;
        let mut reg_503: Word = 0u64;
        let mut reg_504: mmmfloat = 0.0 as mmmfloat;
        let mut reg_505: mmmfloat = 0.0 as mmmfloat;
        let mut reg_506: mmmfloat = 0.0 as mmmfloat;
        let mut reg_507: mmmfloat = 0.0 as mmmfloat;
        let mut reg_508: mmmfloat = 0.0 as mmmfloat;
        let mut reg_509: mmmfloat = 0.0 as mmmfloat;
        let mut reg_510: Word = 0u64;
        let mut reg_511: mmmfloat = 0.0 as mmmfloat;
        let mut reg_512: mmmfloat = 0.0 as mmmfloat;
        let mut reg_513: mmmfloat = 0.0 as mmmfloat;
        let mut reg_514: mmmfloat = 0.0 as mmmfloat;
        let mut reg_515: mmmfloat = 0.0 as mmmfloat;
        let mut reg_516: mmmfloat = 0.0 as mmmfloat;
        let mut reg_517: Word = 0u64;
        let mut reg_518: mmmfloat = 0.0 as mmmfloat;
        let mut reg_519: mmmfloat = 0.0 as mmmfloat;
        let mut reg_520: mmmfloat = 0.0 as mmmfloat;
        let mut reg_521: mmmfloat = 0.0 as mmmfloat;
        let mut reg_522: mmmfloat = 0.0 as mmmfloat;
        let mut reg_523: mmmfloat = 0.0 as mmmfloat;
        let mut reg_524: Word = 0u64;
        let mut reg_525: mmmfloat = 0.0 as mmmfloat;
        let mut reg_526: mmmfloat = 0.0 as mmmfloat;
        let mut reg_527: mmmfloat = 0.0 as mmmfloat;
        let mut reg_528: mmmfloat = 0.0 as mmmfloat;
        let mut reg_529: mmmfloat = 0.0 as mmmfloat;
        let mut reg_530: mmmfloat = 0.0 as mmmfloat;
        let mut reg_531: Word = 0u64;
        let mut reg_532: mmmfloat = 0.0 as mmmfloat;
        let mut reg_533: mmmfloat = 0.0 as mmmfloat;
        let mut reg_534: mmmfloat = 0.0 as mmmfloat;
        let mut reg_535: mmmfloat = 0.0 as mmmfloat;
        let mut reg_536: mmmfloat = 0.0 as mmmfloat;
        let mut reg_537: mmmfloat = 0.0 as mmmfloat;
        let mut reg_538: Word = 0u64;
        let mut reg_539: mmmfloat = 0.0 as mmmfloat;
        let mut reg_540: mmmfloat = 0.0 as mmmfloat;
        let mut reg_541: mmmfloat = 0.0 as mmmfloat;
        let mut reg_542: mmmfloat = 0.0 as mmmfloat;
        let mut reg_543: mmmfloat = 0.0 as mmmfloat;
        let mut reg_544: mmmfloat = 0.0 as mmmfloat;
        let mut reg_545: Word = 0u64;
        let mut reg_546: mmmfloat = 0.0 as mmmfloat;
        let mut reg_547: mmmfloat = 0.0 as mmmfloat;
        let mut reg_548: mmmfloat = 0.0 as mmmfloat;
        let mut reg_549: mmmfloat = 0.0 as mmmfloat;
        let mut reg_550: mmmfloat = 0.0 as mmmfloat;
        let mut reg_551: mmmfloat = 0.0 as mmmfloat;
        let mut reg_552: Word = 0u64;
        let mut reg_553: mmmfloat = 0.0 as mmmfloat;
        let mut reg_554: mmmfloat = 0.0 as mmmfloat;
        let mut reg_555: mmmfloat = 0.0 as mmmfloat;
        let mut reg_556: mmmfloat = 0.0 as mmmfloat;
        let mut reg_557: mmmfloat = 0.0 as mmmfloat;
        let mut reg_558: mmmfloat = 0.0 as mmmfloat;
        let mut reg_559: Word = 0u64;
        let mut reg_560: mmmfloat = 0.0 as mmmfloat;
        let mut reg_561: mmmfloat = 0.0 as mmmfloat;
        let mut reg_562: mmmfloat = 0.0 as mmmfloat;
        let mut reg_563: mmmfloat = 0.0 as mmmfloat;
        let mut reg_564: mmmfloat = 0.0 as mmmfloat;
        let mut reg_565: mmmfloat = 0.0 as mmmfloat;
        let mut reg_566: Word = 0u64;
        let mut reg_567: mmmfloat = 0.0 as mmmfloat;
        let mut reg_568: mmmfloat = 0.0 as mmmfloat;
        let mut reg_569: mmmfloat = 0.0 as mmmfloat;
        let mut reg_570: mmmfloat = 0.0 as mmmfloat;
        let mut reg_571: mmmfloat = 0.0 as mmmfloat;
        let mut reg_572: mmmfloat = 0.0 as mmmfloat;
        let mut reg_573: Word = 0u64;
        let mut reg_574: mmmfloat = 0.0 as mmmfloat;
        let mut reg_575: mmmfloat = 0.0 as mmmfloat;
        let mut reg_576: mmmfloat = 0.0 as mmmfloat;
        let mut reg_577: mmmfloat = 0.0 as mmmfloat;
        let mut reg_578: mmmfloat = 0.0 as mmmfloat;
        let mut reg_579: mmmfloat = 0.0 as mmmfloat;
        let mut reg_580: Word = 0u64;
        let mut reg_581: mmmfloat = 0.0 as mmmfloat;
        let mut reg_582: mmmfloat = 0.0 as mmmfloat;
        let mut reg_583: mmmfloat = 0.0 as mmmfloat;
        let mut reg_584: mmmfloat = 0.0 as mmmfloat;
        let mut reg_585: mmmfloat = 0.0 as mmmfloat;
        let mut reg_586: mmmfloat = 0.0 as mmmfloat;
        let mut reg_587: Word = 0u64;
        let mut reg_588: mmmfloat = 0.0 as mmmfloat;
        let mut reg_589: mmmfloat = 0.0 as mmmfloat;
        let mut reg_590: mmmfloat = 0.0 as mmmfloat;
        let mut reg_591: mmmfloat = 0.0 as mmmfloat;
        let mut reg_592: mmmfloat = 0.0 as mmmfloat;
        let mut reg_593: mmmfloat = 0.0 as mmmfloat;
        let mut reg_594: Word = 0u64;
        let mut reg_595: mmmfloat = 0.0 as mmmfloat;
        let mut reg_596: mmmfloat = 0.0 as mmmfloat;
        let mut reg_597: mmmfloat = 0.0 as mmmfloat;
        let mut reg_598: mmmfloat = 0.0 as mmmfloat;
        let mut reg_599: mmmfloat = 0.0 as mmmfloat;
        let mut reg_600: mmmfloat = 0.0 as mmmfloat;
        let mut reg_601: Word = 0u64;
        let mut reg_602: mmmfloat = 0.0 as mmmfloat;
        let mut reg_603: mmmfloat = 0.0 as mmmfloat;
        let mut reg_604: mmmfloat = 0.0 as mmmfloat;
        let mut reg_605: mmmfloat = 0.0 as mmmfloat;
        let mut reg_606: mmmfloat = 0.0 as mmmfloat;
        let mut reg_607: mmmfloat = 0.0 as mmmfloat;
        let mut reg_608: Word = 0u64;
        let mut reg_609: mmmfloat = 0.0 as mmmfloat;
        let mut reg_610: mmmfloat = 0.0 as mmmfloat;
        let mut reg_611: mmmfloat = 0.0 as mmmfloat;
        let mut reg_612: mmmfloat = 0.0 as mmmfloat;
        let mut reg_613: mmmfloat = 0.0 as mmmfloat;
        let mut reg_614: mmmfloat = 0.0 as mmmfloat;
        let mut reg_615: Word = 0u64;
        let mut reg_616: mmmfloat = 0.0 as mmmfloat;
        let mut reg_617: mmmfloat = 0.0 as mmmfloat;
        let mut reg_618: mmmfloat = 0.0 as mmmfloat;
        let mut reg_619: mmmfloat = 0.0 as mmmfloat;
        let mut reg_620: mmmfloat = 0.0 as mmmfloat;
        let mut reg_621: mmmfloat = 0.0 as mmmfloat;
        let mut reg_622: Word = 0u64;
        let mut reg_623: mmmfloat = 0.0 as mmmfloat;
        let mut reg_624: mmmfloat = 0.0 as mmmfloat;
        let mut reg_625: mmmfloat = 0.0 as mmmfloat;
        let mut reg_626: mmmfloat = 0.0 as mmmfloat;
        let mut reg_627: mmmfloat = 0.0 as mmmfloat;
        let mut reg_628: mmmfloat = 0.0 as mmmfloat;
        let mut reg_629: Word = 0u64;
        let mut reg_630: mmmfloat = 0.0 as mmmfloat;
        let mut reg_631: mmmfloat = 0.0 as mmmfloat;
        let mut reg_632: mmmfloat = 0.0 as mmmfloat;
        let mut reg_633: mmmfloat = 0.0 as mmmfloat;
        let mut reg_634: mmmfloat = 0.0 as mmmfloat;
        let mut reg_635: mmmfloat = 0.0 as mmmfloat;
        let mut reg_636: Word = 0u64;
        let mut reg_637: mmmfloat = 0.0 as mmmfloat;
        let mut reg_638: mmmfloat = 0.0 as mmmfloat;
        let mut reg_639: mmmfloat = 0.0 as mmmfloat;
        let mut reg_640: mmmfloat = 0.0 as mmmfloat;
        let mut reg_641: mmmfloat = 0.0 as mmmfloat;
        let mut reg_642: mmmfloat = 0.0 as mmmfloat;
        let mut reg_643: Word = 0u64;
        let mut reg_644: mmmfloat = 0.0 as mmmfloat;
        let mut reg_645: mmmfloat = 0.0 as mmmfloat;
        let mut reg_646: mmmfloat = 0.0 as mmmfloat;
        let mut reg_647: mmmfloat = 0.0 as mmmfloat;
        let mut reg_648: mmmfloat = 0.0 as mmmfloat;
        let mut reg_649: mmmfloat = 0.0 as mmmfloat;
        let mut reg_650: Word = 0u64;
        let mut reg_651: mmmfloat = 0.0 as mmmfloat;
        let mut reg_652: mmmfloat = 0.0 as mmmfloat;
        let mut reg_653: mmmfloat = 0.0 as mmmfloat;
        let mut reg_654: mmmfloat = 0.0 as mmmfloat;
        let mut reg_655: mmmfloat = 0.0 as mmmfloat;
        let mut reg_656: mmmfloat = 0.0 as mmmfloat;
        let mut reg_657: Word = 0u64;
        let mut reg_658: mmmfloat = 0.0 as mmmfloat;
        let mut reg_659: mmmfloat = 0.0 as mmmfloat;
        let mut reg_660: mmmfloat = 0.0 as mmmfloat;
        let mut reg_661: mmmfloat = 0.0 as mmmfloat;
        let mut reg_662: mmmfloat = 0.0 as mmmfloat;
        let mut reg_663: mmmfloat = 0.0 as mmmfloat;
        let mut reg_664: Word = 0u64;
        let mut reg_665: mmmfloat = 0.0 as mmmfloat;
        let mut reg_666: mmmfloat = 0.0 as mmmfloat;
        let mut reg_667: mmmfloat = 0.0 as mmmfloat;
        let mut reg_668: mmmfloat = 0.0 as mmmfloat;
        let mut reg_669: mmmfloat = 0.0 as mmmfloat;
        let mut reg_670: mmmfloat = 0.0 as mmmfloat;
        let mut reg_671: Word = 0u64;
        let mut reg_672: mmmfloat = 0.0 as mmmfloat;
        let mut reg_673: mmmfloat = 0.0 as mmmfloat;
        let mut reg_674: mmmfloat = 0.0 as mmmfloat;
        let mut reg_675: mmmfloat = 0.0 as mmmfloat;
        let mut reg_676: mmmfloat = 0.0 as mmmfloat;
        let mut reg_677: mmmfloat = 0.0 as mmmfloat;
        let mut reg_678: Word = 0u64;
        let mut reg_679: mmmfloat = 0.0 as mmmfloat;
        let mut reg_680: mmmfloat = 0.0 as mmmfloat;
        let mut reg_681: mmmfloat = 0.0 as mmmfloat;
        let mut reg_682: mmmfloat = 0.0 as mmmfloat;
        let mut reg_683: mmmfloat = 0.0 as mmmfloat;
        let mut reg_684: mmmfloat = 0.0 as mmmfloat;
        let mut reg_685: Word = 0u64;
        let mut reg_686: mmmfloat = 0.0 as mmmfloat;
        let mut reg_687: mmmfloat = 0.0 as mmmfloat;
        let mut reg_688: mmmfloat = 0.0 as mmmfloat;
        let mut reg_689: mmmfloat = 0.0 as mmmfloat;
        let mut reg_690: mmmfloat = 0.0 as mmmfloat;
        let mut reg_691: mmmfloat = 0.0 as mmmfloat;
        let mut reg_692: Word = 0u64;
        let mut reg_693: mmmfloat = 0.0 as mmmfloat;
        let mut reg_694: mmmfloat = 0.0 as mmmfloat;
        let mut reg_695: mmmfloat = 0.0 as mmmfloat;
        let mut reg_696: mmmfloat = 0.0 as mmmfloat;
        let mut reg_697: mmmfloat = 0.0 as mmmfloat;
        let mut reg_698: mmmfloat = 0.0 as mmmfloat;
        let mut reg_699: Word = 0u64;
        let mut reg_700: mmmfloat = 0.0 as mmmfloat;
        let mut reg_701: mmmfloat = 0.0 as mmmfloat;
        let mut reg_702: mmmfloat = 0.0 as mmmfloat;
        let mut reg_703: mmmfloat = 0.0 as mmmfloat;
        let mut reg_704: mmmfloat = 0.0 as mmmfloat;
        let mut reg_705: mmmfloat = 0.0 as mmmfloat;
        let mut reg_706: Word = 0u64;
        let mut reg_707: mmmfloat = 0.0 as mmmfloat;
        let mut reg_708: mmmfloat = 0.0 as mmmfloat;
        let mut reg_709: mmmfloat = 0.0 as mmmfloat;
        let mut reg_710: mmmfloat = 0.0 as mmmfloat;
        let mut reg_711: mmmfloat = 0.0 as mmmfloat;
        let mut reg_712: mmmfloat = 0.0 as mmmfloat;
        let mut reg_713: Word = 0u64;
        let mut reg_714: mmmfloat = 0.0 as mmmfloat;
        let mut reg_715: mmmfloat = 0.0 as mmmfloat;
        let mut reg_716: mmmfloat = 0.0 as mmmfloat;
        let mut reg_717: mmmfloat = 0.0 as mmmfloat;
        let mut reg_718: mmmfloat = 0.0 as mmmfloat;
        let mut reg_719: mmmfloat = 0.0 as mmmfloat;
        let mut reg_720: Word = 0u64;
        let mut reg_721: mmmfloat = 0.0 as mmmfloat;
        let mut reg_722: mmmfloat = 0.0 as mmmfloat;
        let mut reg_723: mmmfloat = 0.0 as mmmfloat;
        let mut reg_724: mmmfloat = 0.0 as mmmfloat;
        let mut reg_725: mmmfloat = 0.0 as mmmfloat;
        let mut reg_726: mmmfloat = 0.0 as mmmfloat;
        let mut reg_727: Word = 0u64;
        let mut reg_728: mmmfloat = 0.0 as mmmfloat;
        let mut reg_729: mmmfloat = 0.0 as mmmfloat;
        let mut reg_730: mmmfloat = 0.0 as mmmfloat;
        let mut reg_731: mmmfloat = 0.0 as mmmfloat;
        let mut reg_732: mmmfloat = 0.0 as mmmfloat;
        let mut reg_733: mmmfloat = 0.0 as mmmfloat;
        let mut reg_734: Word = 0u64;
        let mut reg_735: mmmfloat = 0.0 as mmmfloat;
        let mut reg_736: mmmfloat = 0.0 as mmmfloat;
        let mut reg_737: mmmfloat = 0.0 as mmmfloat;
        let mut reg_738: mmmfloat = 0.0 as mmmfloat;
        let mut reg_739: mmmfloat = 0.0 as mmmfloat;
        let mut reg_740: mmmfloat = 0.0 as mmmfloat;
        let mut reg_741: Word = 0u64;
        let mut reg_742: mmmfloat = 0.0 as mmmfloat;
        let mut reg_743: mmmfloat = 0.0 as mmmfloat;
        let mut reg_744: mmmfloat = 0.0 as mmmfloat;
        let mut reg_745: mmmfloat = 0.0 as mmmfloat;
        let mut reg_746: mmmfloat = 0.0 as mmmfloat;
        let mut reg_747: mmmfloat = 0.0 as mmmfloat;
        let mut reg_748: Word = 0u64;
        let mut reg_749: mmmfloat = 0.0 as mmmfloat;
        let mut reg_750: mmmfloat = 0.0 as mmmfloat;
        let mut reg_751: mmmfloat = 0.0 as mmmfloat;
        let mut reg_752: mmmfloat = 0.0 as mmmfloat;
        let mut reg_753: mmmfloat = 0.0 as mmmfloat;
        let mut reg_754: mmmfloat = 0.0 as mmmfloat;
        let mut reg_755: Word = 0u64;
        let mut reg_756: mmmfloat = 0.0 as mmmfloat;
        let mut reg_757: mmmfloat = 0.0 as mmmfloat;
        let mut reg_758: mmmfloat = 0.0 as mmmfloat;
        let mut reg_759: mmmfloat = 0.0 as mmmfloat;
        let mut reg_760: mmmfloat = 0.0 as mmmfloat;
        let mut reg_761: mmmfloat = 0.0 as mmmfloat;
        let mut reg_762: Word = 0u64;
        let mut reg_763: mmmfloat = 0.0 as mmmfloat;
        let mut reg_764: mmmfloat = 0.0 as mmmfloat;
        let mut reg_765: mmmfloat = 0.0 as mmmfloat;
        let mut reg_766: mmmfloat = 0.0 as mmmfloat;
        let mut reg_767: mmmfloat = 0.0 as mmmfloat;
        let mut reg_768: mmmfloat = 0.0 as mmmfloat;
        let mut reg_769: Word = 0u64;
        let mut reg_770: mmmfloat = 0.0 as mmmfloat;
        let mut reg_771: mmmfloat = 0.0 as mmmfloat;
        let mut reg_772: mmmfloat = 0.0 as mmmfloat;
        let mut reg_773: mmmfloat = 0.0 as mmmfloat;
        let mut reg_774: mmmfloat = 0.0 as mmmfloat;
        let mut reg_775: mmmfloat = 0.0 as mmmfloat;
        let mut reg_776: Word = 0u64;
        let mut reg_777: mmmfloat = 0.0 as mmmfloat;
        let mut reg_778: mmmfloat = 0.0 as mmmfloat;
        let mut reg_779: mmmfloat = 0.0 as mmmfloat;
        let mut reg_780: mmmfloat = 0.0 as mmmfloat;
        let mut reg_781: mmmfloat = 0.0 as mmmfloat;
        let mut reg_782: mmmfloat = 0.0 as mmmfloat;
        let mut reg_783: Word = 0u64;
        let mut reg_784: mmmfloat = 0.0 as mmmfloat;
        let mut reg_785: mmmfloat = 0.0 as mmmfloat;
        let mut reg_786: mmmfloat = 0.0 as mmmfloat;
        let mut reg_787: mmmfloat = 0.0 as mmmfloat;
        let mut reg_788: mmmfloat = 0.0 as mmmfloat;
        let mut reg_789: mmmfloat = 0.0 as mmmfloat;
        let mut reg_790: Word = 0u64;
        let mut reg_791: mmmfloat = 0.0 as mmmfloat;
        let mut reg_792: mmmfloat = 0.0 as mmmfloat;
        let mut reg_793: mmmfloat = 0.0 as mmmfloat;
        let mut reg_794: mmmfloat = 0.0 as mmmfloat;
        let mut reg_795: mmmfloat = 0.0 as mmmfloat;
        let mut reg_796: mmmfloat = 0.0 as mmmfloat;
        let mut reg_797: Word = 0u64;
        let mut reg_798: mmmfloat = 0.0 as mmmfloat;
        let mut reg_799: mmmfloat = 0.0 as mmmfloat;
        let mut reg_800: mmmfloat = 0.0 as mmmfloat;
        let mut reg_801: mmmfloat = 0.0 as mmmfloat;
        let mut reg_802: mmmfloat = 0.0 as mmmfloat;
        let mut reg_803: mmmfloat = 0.0 as mmmfloat;
        let mut reg_804: Word = 0u64;
        let mut reg_805: mmmfloat = 0.0 as mmmfloat;
        let mut reg_806: mmmfloat = 0.0 as mmmfloat;
        let mut reg_807: mmmfloat = 0.0 as mmmfloat;
        let mut reg_808: mmmfloat = 0.0 as mmmfloat;
        let mut reg_809: mmmfloat = 0.0 as mmmfloat;
        let mut reg_810: mmmfloat = 0.0 as mmmfloat;
        let mut reg_811: Word = 0u64;
        let mut reg_812: mmmfloat = 0.0 as mmmfloat;
        let mut reg_813: mmmfloat = 0.0 as mmmfloat;
        let mut reg_814: mmmfloat = 0.0 as mmmfloat;
        let mut reg_815: mmmfloat = 0.0 as mmmfloat;
        let mut reg_816: mmmfloat = 0.0 as mmmfloat;
        let mut reg_817: mmmfloat = 0.0 as mmmfloat;
        let mut reg_818: Word = 0u64;
        let mut reg_819: mmmfloat = 0.0 as mmmfloat;
        let mut reg_820: mmmfloat = 0.0 as mmmfloat;
        let mut reg_821: mmmfloat = 0.0 as mmmfloat;
        let mut reg_822: mmmfloat = 0.0 as mmmfloat;
        let mut reg_823: mmmfloat = 0.0 as mmmfloat;
        let mut reg_824: mmmfloat = 0.0 as mmmfloat;
        let mut reg_825: Word = 0u64;
        let mut reg_826: mmmfloat = 0.0 as mmmfloat;
        let mut reg_827: mmmfloat = 0.0 as mmmfloat;
        let mut reg_828: mmmfloat = 0.0 as mmmfloat;
        let mut reg_829: mmmfloat = 0.0 as mmmfloat;
        let mut reg_830: mmmfloat = 0.0 as mmmfloat;
        let mut reg_831: mmmfloat = 0.0 as mmmfloat;
        let mut reg_832: Word = 0u64;
        let mut reg_833: mmmfloat = 0.0 as mmmfloat;
        let mut reg_834: mmmfloat = 0.0 as mmmfloat;
        let mut reg_835: mmmfloat = 0.0 as mmmfloat;
        let mut reg_836: mmmfloat = 0.0 as mmmfloat;
        let mut reg_837: mmmfloat = 0.0 as mmmfloat;
        let mut reg_838: mmmfloat = 0.0 as mmmfloat;
        let mut reg_839: Word = 0u64;
        let mut reg_840: mmmfloat = 0.0 as mmmfloat;
        let mut reg_841: mmmfloat = 0.0 as mmmfloat;
        let mut reg_842: mmmfloat = 0.0 as mmmfloat;
        let mut reg_843: mmmfloat = 0.0 as mmmfloat;
        let mut reg_844: mmmfloat = 0.0 as mmmfloat;
        let mut reg_845: mmmfloat = 0.0 as mmmfloat;
        let mut reg_846: Word = 0u64;
        let mut reg_847: mmmfloat = 0.0 as mmmfloat;
        let mut reg_848: mmmfloat = 0.0 as mmmfloat;
        let mut reg_849: mmmfloat = 0.0 as mmmfloat;
        let mut reg_850: mmmfloat = 0.0 as mmmfloat;
        let mut reg_851: mmmfloat = 0.0 as mmmfloat;
        let mut reg_852: mmmfloat = 0.0 as mmmfloat;
        let mut reg_853: Word = 0u64;
        let mut reg_854: mmmfloat = 0.0 as mmmfloat;
        let mut reg_855: mmmfloat = 0.0 as mmmfloat;
        let mut reg_856: mmmfloat = 0.0 as mmmfloat;
        let mut reg_857: mmmfloat = 0.0 as mmmfloat;
        let mut reg_858: mmmfloat = 0.0 as mmmfloat;
        let mut reg_859: mmmfloat = 0.0 as mmmfloat;
        let mut reg_860: Word = 0u64;
        let mut reg_861: mmmfloat = 0.0 as mmmfloat;
        let mut reg_862: mmmfloat = 0.0 as mmmfloat;
        let mut reg_863: mmmfloat = 0.0 as mmmfloat;
        let mut reg_864: mmmfloat = 0.0 as mmmfloat;
        let mut reg_865: mmmfloat = 0.0 as mmmfloat;
        let mut reg_866: mmmfloat = 0.0 as mmmfloat;
        let mut reg_867: Word = 0u64;
        let mut reg_868: mmmfloat = 0.0 as mmmfloat;
        let mut reg_869: mmmfloat = 0.0 as mmmfloat;
        let mut reg_870: mmmfloat = 0.0 as mmmfloat;
        let mut reg_871: mmmfloat = 0.0 as mmmfloat;
        let mut reg_872: Word = 0u64;
        let mut reg_873: mmmfloat = 0.0 as mmmfloat;
        let mut reg_874: mmmfloat = 0.0 as mmmfloat;
        let mut reg_875: mmmfloat = 0.0 as mmmfloat;
        let mut reg_876: mmmfloat = 0.0 as mmmfloat;
        let mut reg_877: mmmfloat = 0.0 as mmmfloat;
        let mut reg_878: mmmfloat = 0.0 as mmmfloat;
        let mut reg_879: mmmfloat = 0.0 as mmmfloat;
        let mut reg_880: mmmfloat = 0.0 as mmmfloat;
        let mut reg_881: mmmfloat = 0.0 as mmmfloat;
        let mut reg_882: mmmfloat = 0.0 as mmmfloat;
        let mut reg_883: mmmfloat = 0.0 as mmmfloat;
        let mut reg_884: mmmfloat = 0.0 as mmmfloat;
        let mut reg_885: mmmfloat = 0.0 as mmmfloat;
        let mut reg_886: mmmfloat = 0.0 as mmmfloat;
        let mut reg_887: mmmfloat = 0.0 as mmmfloat;
        let mut reg_888: mmmfloat = 0.0 as mmmfloat;
        let mut reg_889: mmmfloat = 0.0 as mmmfloat;
        let mut reg_890: mmmfloat = 0.0 as mmmfloat;
        let mut reg_891: mmmfloat = 0.0 as mmmfloat;
        let mut reg_892: mmmfloat = 0.0 as mmmfloat;
        let mut reg_893: mmmfloat = 0.0 as mmmfloat;
        let mut reg_894: mmmfloat = 0.0 as mmmfloat;
        let mut reg_895: mmmfloat = 0.0 as mmmfloat;
        let mut reg_896: mmmfloat = 0.0 as mmmfloat;
        let mut reg_897: mmmfloat = 0.0 as mmmfloat;
        let mut reg_898: mmmfloat = 0.0 as mmmfloat;
        let mut reg_899: mmmfloat = 0.0 as mmmfloat;
        let mut reg_900: mmmfloat = 0.0 as mmmfloat;
        let mut reg_901: mmmfloat = 0.0 as mmmfloat;
        let mut reg_902: mmmfloat = 0.0 as mmmfloat;
        let mut reg_903: mmmfloat = 0.0 as mmmfloat;
        let mut reg_904: mmmfloat = 0.0 as mmmfloat;
        let mut reg_905: mmmfloat = 0.0 as mmmfloat;
        let mut reg_906: mmmfloat = 0.0 as mmmfloat;
        let mut reg_907: mmmfloat = 0.0 as mmmfloat;
        let mut reg_908: mmmfloat = 0.0 as mmmfloat;
        let mut reg_909: mmmfloat = 0.0 as mmmfloat;
        let mut reg_910: mmmfloat = 0.0 as mmmfloat;
        let mut reg_911: mmmfloat = 0.0 as mmmfloat;
        let mut reg_912: mmmfloat = 0.0 as mmmfloat;
        let mut reg_913: mmmfloat = 0.0 as mmmfloat;
        let mut reg_914: mmmfloat = 0.0 as mmmfloat;
        let mut reg_915: mmmfloat = 0.0 as mmmfloat;
        let mut reg_916: mmmfloat = 0.0 as mmmfloat;
        let mut reg_917: mmmfloat = 0.0 as mmmfloat;
        let mut reg_918: mmmfloat = 0.0 as mmmfloat;
        let mut reg_919: mmmfloat = 0.0 as mmmfloat;
        let mut reg_920: mmmfloat = 0.0 as mmmfloat;
        let mut reg_921: mmmfloat = 0.0 as mmmfloat;
        let mut reg_922: mmmfloat = 0.0 as mmmfloat;
        let mut reg_923: mmmfloat = 0.0 as mmmfloat;
        let mut reg_924: mmmfloat = 0.0 as mmmfloat;
        let mut reg_925: mmmfloat = 0.0 as mmmfloat;
        let mut reg_926: mmmfloat = 0.0 as mmmfloat;
        let mut reg_927: mmmfloat = 0.0 as mmmfloat;
        let mut reg_928: mmmfloat = 0.0 as mmmfloat;
        let mut reg_929: mmmfloat = 0.0 as mmmfloat;
        let mut reg_930: mmmfloat = 0.0 as mmmfloat;
        let mut reg_931: mmmfloat = 0.0 as mmmfloat;
        let mut reg_932: mmmfloat = 0.0 as mmmfloat;
        let mut reg_933: mmmfloat = 0.0 as mmmfloat;
        let mut reg_934: mmmfloat = 0.0 as mmmfloat;
        let mut reg_935: mmmfloat = 0.0 as mmmfloat;
        let mut reg_936: mmmfloat = 0.0 as mmmfloat;
        let mut reg_937: mmmfloat = 0.0 as mmmfloat;
        let mut reg_938: mmmfloat = 0.0 as mmmfloat;
        let mut reg_939: mmmfloat = 0.0 as mmmfloat;
        let mut reg_940: mmmfloat = 0.0 as mmmfloat;
        let mut reg_941: mmmfloat = 0.0 as mmmfloat;
        let mut reg_942: mmmfloat = 0.0 as mmmfloat;
        let mut reg_943: mmmfloat = 0.0 as mmmfloat;
        let mut reg_944: mmmfloat = 0.0 as mmmfloat;
        let mut reg_945: mmmfloat = 0.0 as mmmfloat;
        let mut reg_946: mmmfloat = 0.0 as mmmfloat;
        let mut reg_947: mmmfloat = 0.0 as mmmfloat;
        let mut reg_948: mmmfloat = 0.0 as mmmfloat;
        let mut reg_949: mmmfloat = 0.0 as mmmfloat;
        let mut reg_950: mmmfloat = 0.0 as mmmfloat;
        let mut reg_951: mmmfloat = 0.0 as mmmfloat;
        let mut reg_952: mmmfloat = 0.0 as mmmfloat;
        let mut reg_953: mmmfloat = 0.0 as mmmfloat;
        let mut reg_954: mmmfloat = 0.0 as mmmfloat;
        let mut reg_955: mmmfloat = 0.0 as mmmfloat;
        let mut reg_956: mmmfloat = 0.0 as mmmfloat;
        let mut reg_957: mmmfloat = 0.0 as mmmfloat;
        let mut reg_958: mmmfloat = 0.0 as mmmfloat;
        let mut reg_959: mmmfloat = 0.0 as mmmfloat;
        let mut reg_960: mmmfloat = 0.0 as mmmfloat;
        let mut reg_961: mmmfloat = 0.0 as mmmfloat;
        let mut reg_962: mmmfloat = 0.0 as mmmfloat;
        let mut reg_963: mmmfloat = 0.0 as mmmfloat;
        let mut reg_964: mmmfloat = 0.0 as mmmfloat;
        let mut reg_965: mmmfloat = 0.0 as mmmfloat;
        let mut reg_966: mmmfloat = 0.0 as mmmfloat;
        let mut reg_967: mmmfloat = 0.0 as mmmfloat;
        let mut reg_968: mmmfloat = 0.0 as mmmfloat;
        let mut reg_969: mmmfloat = 0.0 as mmmfloat;
        let mut reg_970: mmmfloat = 0.0 as mmmfloat;
        let mut reg_971: mmmfloat = 0.0 as mmmfloat;
        let mut reg_972: mmmfloat = 0.0 as mmmfloat;
        let mut reg_973: mmmfloat = 0.0 as mmmfloat;
        let mut reg_974: Word = 0u64;
        let mut reg_975: Word = 0u64;
        let mut reg_976: Word = 0u64;
        let mut reg_977: mmmfloat = 0.0 as mmmfloat;
        let mut reg_978: Word = 0u64;
        let mut reg_979: Word = 0u64;
        let mut reg_980: mmmfloat = 0.0 as mmmfloat;
        let mut reg_981: Word = 0u64;
        let mut reg_982: Word = 0u64;
        let mut reg_983: Word = 0u64;
        let mut stack_alloc_974 = [0u64; 1];
        let mut stack_alloc_976 = [0u64; 2];
        reg_171 = 50.0 as mmmfloat;
        reg_172 = 100.0 as mmmfloat;
        reg_173 = reg_171 * reg_172;
        reg_174 = 27u64;
        let call_result = self.osc(reg_173);
        reg_175 = call_result;
        reg_176 = 100.0 as mmmfloat;
        reg_177 = reg_175 / reg_176;
        reg_178 = 50.0 as mmmfloat;
        reg_179 = 99.0 as mmmfloat;
        reg_180 = reg_178 * reg_179;
        self.get_current_statestorage().push_pos(1usize);
        reg_181 = 27u64;
        let call_result = self.osc(reg_180);
        reg_182 = call_result;
        reg_183 = 99.0 as mmmfloat;
        reg_184 = reg_182 / reg_183;
        reg_185 = 50.0 as mmmfloat;
        reg_186 = 98.0 as mmmfloat;
        reg_187 = reg_185 * reg_186;
        self.get_current_statestorage().push_pos(1usize);
        reg_188 = 27u64;
        let call_result = self.osc(reg_187);
        reg_189 = call_result;
        reg_190 = 98.0 as mmmfloat;
        reg_191 = reg_189 / reg_190;
        reg_192 = 50.0 as mmmfloat;
        reg_193 = 97.0 as mmmfloat;
        reg_194 = reg_192 * reg_193;
        self.get_current_statestorage().push_pos(1usize);
        reg_195 = 27u64;
        let call_result = self.osc(reg_194);
        reg_196 = call_result;
        reg_197 = 97.0 as mmmfloat;
        reg_198 = reg_196 / reg_197;
        reg_199 = 50.0 as mmmfloat;
        reg_200 = 96.0 as mmmfloat;
        reg_201 = reg_199 * reg_200;
        self.get_current_statestorage().push_pos(1usize);
        reg_202 = 27u64;
        let call_result = self.osc(reg_201);
        reg_203 = call_result;
        reg_204 = 96.0 as mmmfloat;
        reg_205 = reg_203 / reg_204;
        reg_206 = 50.0 as mmmfloat;
        reg_207 = 95.0 as mmmfloat;
        reg_208 = reg_206 * reg_207;
        self.get_current_statestorage().push_pos(1usize);
        reg_209 = 27u64;
        let call_result = self.osc(reg_208);
        reg_210 = call_result;
        reg_211 = 95.0 as mmmfloat;
        reg_212 = reg_210 / reg_211;
        reg_213 = 50.0 as mmmfloat;
        reg_214 = 94.0 as mmmfloat;
        reg_215 = reg_213 * reg_214;
        self.get_current_statestorage().push_pos(1usize);
        reg_216 = 27u64;
        let call_result = self.osc(reg_215);
        reg_217 = call_result;
        reg_218 = 94.0 as mmmfloat;
        reg_219 = reg_217 / reg_218;
        reg_220 = 50.0 as mmmfloat;
        reg_221 = 93.0 as mmmfloat;
        reg_222 = reg_220 * reg_221;
        self.get_current_statestorage().push_pos(1usize);
        reg_223 = 27u64;
        let call_result = self.osc(reg_222);
        reg_224 = call_result;
        reg_225 = 93.0 as mmmfloat;
        reg_226 = reg_224 / reg_225;
        reg_227 = 50.0 as mmmfloat;
        reg_228 = 92.0 as mmmfloat;
        reg_229 = reg_227 * reg_228;
        self.get_current_statestorage().push_pos(1usize);
        reg_230 = 27u64;
        let call_result = self.osc(reg_229);
        reg_231 = call_result;
        reg_232 = 92.0 as mmmfloat;
        reg_233 = reg_231 / reg_232;
        reg_234 = 50.0 as mmmfloat;
        reg_235 = 91.0 as mmmfloat;
        reg_236 = reg_234 * reg_235;
        self.get_current_statestorage().push_pos(1usize);
        reg_237 = 27u64;
        let call_result = self.osc(reg_236);
        reg_238 = call_result;
        reg_239 = 91.0 as mmmfloat;
        reg_240 = reg_238 / reg_239;
        reg_241 = 50.0 as mmmfloat;
        reg_242 = 90.0 as mmmfloat;
        reg_243 = reg_241 * reg_242;
        self.get_current_statestorage().push_pos(1usize);
        reg_244 = 27u64;
        let call_result = self.osc(reg_243);
        reg_245 = call_result;
        reg_246 = 90.0 as mmmfloat;
        reg_247 = reg_245 / reg_246;
        reg_248 = 50.0 as mmmfloat;
        reg_249 = 89.0 as mmmfloat;
        reg_250 = reg_248 * reg_249;
        self.get_current_statestorage().push_pos(1usize);
        reg_251 = 27u64;
        let call_result = self.osc(reg_250);
        reg_252 = call_result;
        reg_253 = 89.0 as mmmfloat;
        reg_254 = reg_252 / reg_253;
        reg_255 = 50.0 as mmmfloat;
        reg_256 = 88.0 as mmmfloat;
        reg_257 = reg_255 * reg_256;
        self.get_current_statestorage().push_pos(1usize);
        reg_258 = 27u64;
        let call_result = self.osc(reg_257);
        reg_259 = call_result;
        reg_260 = 88.0 as mmmfloat;
        reg_261 = reg_259 / reg_260;
        reg_262 = 50.0 as mmmfloat;
        reg_263 = 87.0 as mmmfloat;
        reg_264 = reg_262 * reg_263;
        self.get_current_statestorage().push_pos(1usize);
        reg_265 = 27u64;
        let call_result = self.osc(reg_264);
        reg_266 = call_result;
        reg_267 = 87.0 as mmmfloat;
        reg_268 = reg_266 / reg_267;
        reg_269 = 50.0 as mmmfloat;
        reg_270 = 86.0 as mmmfloat;
        reg_271 = reg_269 * reg_270;
        self.get_current_statestorage().push_pos(1usize);
        reg_272 = 27u64;
        let call_result = self.osc(reg_271);
        reg_273 = call_result;
        reg_274 = 86.0 as mmmfloat;
        reg_275 = reg_273 / reg_274;
        reg_276 = 50.0 as mmmfloat;
        reg_277 = 85.0 as mmmfloat;
        reg_278 = reg_276 * reg_277;
        self.get_current_statestorage().push_pos(1usize);
        reg_279 = 27u64;
        let call_result = self.osc(reg_278);
        reg_280 = call_result;
        reg_281 = 85.0 as mmmfloat;
        reg_282 = reg_280 / reg_281;
        reg_283 = 50.0 as mmmfloat;
        reg_284 = 84.0 as mmmfloat;
        reg_285 = reg_283 * reg_284;
        self.get_current_statestorage().push_pos(1usize);
        reg_286 = 27u64;
        let call_result = self.osc(reg_285);
        reg_287 = call_result;
        reg_288 = 84.0 as mmmfloat;
        reg_289 = reg_287 / reg_288;
        reg_290 = 50.0 as mmmfloat;
        reg_291 = 83.0 as mmmfloat;
        reg_292 = reg_290 * reg_291;
        self.get_current_statestorage().push_pos(1usize);
        reg_293 = 27u64;
        let call_result = self.osc(reg_292);
        reg_294 = call_result;
        reg_295 = 83.0 as mmmfloat;
        reg_296 = reg_294 / reg_295;
        reg_297 = 50.0 as mmmfloat;
        reg_298 = 82.0 as mmmfloat;
        reg_299 = reg_297 * reg_298;
        self.get_current_statestorage().push_pos(1usize);
        reg_300 = 27u64;
        let call_result = self.osc(reg_299);
        reg_301 = call_result;
        reg_302 = 82.0 as mmmfloat;
        reg_303 = reg_301 / reg_302;
        reg_304 = 50.0 as mmmfloat;
        reg_305 = 81.0 as mmmfloat;
        reg_306 = reg_304 * reg_305;
        self.get_current_statestorage().push_pos(1usize);
        reg_307 = 27u64;
        let call_result = self.osc(reg_306);
        reg_308 = call_result;
        reg_309 = 81.0 as mmmfloat;
        reg_310 = reg_308 / reg_309;
        reg_311 = 50.0 as mmmfloat;
        reg_312 = 80.0 as mmmfloat;
        reg_313 = reg_311 * reg_312;
        self.get_current_statestorage().push_pos(1usize);
        reg_314 = 27u64;
        let call_result = self.osc(reg_313);
        reg_315 = call_result;
        reg_316 = 80.0 as mmmfloat;
        reg_317 = reg_315 / reg_316;
        reg_318 = 50.0 as mmmfloat;
        reg_319 = 79.0 as mmmfloat;
        reg_320 = reg_318 * reg_319;
        self.get_current_statestorage().push_pos(1usize);
        reg_321 = 27u64;
        let call_result = self.osc(reg_320);
        reg_322 = call_result;
        reg_323 = 79.0 as mmmfloat;
        reg_324 = reg_322 / reg_323;
        reg_325 = 50.0 as mmmfloat;
        reg_326 = 78.0 as mmmfloat;
        reg_327 = reg_325 * reg_326;
        self.get_current_statestorage().push_pos(1usize);
        reg_328 = 27u64;
        let call_result = self.osc(reg_327);
        reg_329 = call_result;
        reg_330 = 78.0 as mmmfloat;
        reg_331 = reg_329 / reg_330;
        reg_332 = 50.0 as mmmfloat;
        reg_333 = 77.0 as mmmfloat;
        reg_334 = reg_332 * reg_333;
        self.get_current_statestorage().push_pos(1usize);
        reg_335 = 27u64;
        let call_result = self.osc(reg_334);
        reg_336 = call_result;
        reg_337 = 77.0 as mmmfloat;
        reg_338 = reg_336 / reg_337;
        reg_339 = 50.0 as mmmfloat;
        reg_340 = 76.0 as mmmfloat;
        reg_341 = reg_339 * reg_340;
        self.get_current_statestorage().push_pos(1usize);
        reg_342 = 27u64;
        let call_result = self.osc(reg_341);
        reg_343 = call_result;
        reg_344 = 76.0 as mmmfloat;
        reg_345 = reg_343 / reg_344;
        reg_346 = 50.0 as mmmfloat;
        reg_347 = 75.0 as mmmfloat;
        reg_348 = reg_346 * reg_347;
        self.get_current_statestorage().push_pos(1usize);
        reg_349 = 27u64;
        let call_result = self.osc(reg_348);
        reg_350 = call_result;
        reg_351 = 75.0 as mmmfloat;
        reg_352 = reg_350 / reg_351;
        reg_353 = 50.0 as mmmfloat;
        reg_354 = 74.0 as mmmfloat;
        reg_355 = reg_353 * reg_354;
        self.get_current_statestorage().push_pos(1usize);
        reg_356 = 27u64;
        let call_result = self.osc(reg_355);
        reg_357 = call_result;
        reg_358 = 74.0 as mmmfloat;
        reg_359 = reg_357 / reg_358;
        reg_360 = 50.0 as mmmfloat;
        reg_361 = 73.0 as mmmfloat;
        reg_362 = reg_360 * reg_361;
        self.get_current_statestorage().push_pos(1usize);
        reg_363 = 27u64;
        let call_result = self.osc(reg_362);
        reg_364 = call_result;
        reg_365 = 73.0 as mmmfloat;
        reg_366 = reg_364 / reg_365;
        reg_367 = 50.0 as mmmfloat;
        reg_368 = 72.0 as mmmfloat;
        reg_369 = reg_367 * reg_368;
        self.get_current_statestorage().push_pos(1usize);
        reg_370 = 27u64;
        let call_result = self.osc(reg_369);
        reg_371 = call_result;
        reg_372 = 72.0 as mmmfloat;
        reg_373 = reg_371 / reg_372;
        reg_374 = 50.0 as mmmfloat;
        reg_375 = 71.0 as mmmfloat;
        reg_376 = reg_374 * reg_375;
        self.get_current_statestorage().push_pos(1usize);
        reg_377 = 27u64;
        let call_result = self.osc(reg_376);
        reg_378 = call_result;
        reg_379 = 71.0 as mmmfloat;
        reg_380 = reg_378 / reg_379;
        reg_381 = 50.0 as mmmfloat;
        reg_382 = 70.0 as mmmfloat;
        reg_383 = reg_381 * reg_382;
        self.get_current_statestorage().push_pos(1usize);
        reg_384 = 27u64;
        let call_result = self.osc(reg_383);
        reg_385 = call_result;
        reg_386 = 70.0 as mmmfloat;
        reg_387 = reg_385 / reg_386;
        reg_388 = 50.0 as mmmfloat;
        reg_389 = 69.0 as mmmfloat;
        reg_390 = reg_388 * reg_389;
        self.get_current_statestorage().push_pos(1usize);
        reg_391 = 27u64;
        let call_result = self.osc(reg_390);
        reg_392 = call_result;
        reg_393 = 69.0 as mmmfloat;
        reg_394 = reg_392 / reg_393;
        reg_395 = 50.0 as mmmfloat;
        reg_396 = 68.0 as mmmfloat;
        reg_397 = reg_395 * reg_396;
        self.get_current_statestorage().push_pos(1usize);
        reg_398 = 27u64;
        let call_result = self.osc(reg_397);
        reg_399 = call_result;
        reg_400 = 68.0 as mmmfloat;
        reg_401 = reg_399 / reg_400;
        reg_402 = 50.0 as mmmfloat;
        reg_403 = 67.0 as mmmfloat;
        reg_404 = reg_402 * reg_403;
        self.get_current_statestorage().push_pos(1usize);
        reg_405 = 27u64;
        let call_result = self.osc(reg_404);
        reg_406 = call_result;
        reg_407 = 67.0 as mmmfloat;
        reg_408 = reg_406 / reg_407;
        reg_409 = 50.0 as mmmfloat;
        reg_410 = 66.0 as mmmfloat;
        reg_411 = reg_409 * reg_410;
        self.get_current_statestorage().push_pos(1usize);
        reg_412 = 27u64;
        let call_result = self.osc(reg_411);
        reg_413 = call_result;
        reg_414 = 66.0 as mmmfloat;
        reg_415 = reg_413 / reg_414;
        reg_416 = 50.0 as mmmfloat;
        reg_417 = 65.0 as mmmfloat;
        reg_418 = reg_416 * reg_417;
        self.get_current_statestorage().push_pos(1usize);
        reg_419 = 27u64;
        let call_result = self.osc(reg_418);
        reg_420 = call_result;
        reg_421 = 65.0 as mmmfloat;
        reg_422 = reg_420 / reg_421;
        reg_423 = 50.0 as mmmfloat;
        reg_424 = 64.0 as mmmfloat;
        reg_425 = reg_423 * reg_424;
        self.get_current_statestorage().push_pos(1usize);
        reg_426 = 27u64;
        let call_result = self.osc(reg_425);
        reg_427 = call_result;
        reg_428 = 64.0 as mmmfloat;
        reg_429 = reg_427 / reg_428;
        reg_430 = 50.0 as mmmfloat;
        reg_431 = 63.0 as mmmfloat;
        reg_432 = reg_430 * reg_431;
        self.get_current_statestorage().push_pos(1usize);
        reg_433 = 27u64;
        let call_result = self.osc(reg_432);
        reg_434 = call_result;
        reg_435 = 63.0 as mmmfloat;
        reg_436 = reg_434 / reg_435;
        reg_437 = 50.0 as mmmfloat;
        reg_438 = 62.0 as mmmfloat;
        reg_439 = reg_437 * reg_438;
        self.get_current_statestorage().push_pos(1usize);
        reg_440 = 27u64;
        let call_result = self.osc(reg_439);
        reg_441 = call_result;
        reg_442 = 62.0 as mmmfloat;
        reg_443 = reg_441 / reg_442;
        reg_444 = 50.0 as mmmfloat;
        reg_445 = 61.0 as mmmfloat;
        reg_446 = reg_444 * reg_445;
        self.get_current_statestorage().push_pos(1usize);
        reg_447 = 27u64;
        let call_result = self.osc(reg_446);
        reg_448 = call_result;
        reg_449 = 61.0 as mmmfloat;
        reg_450 = reg_448 / reg_449;
        reg_451 = 50.0 as mmmfloat;
        reg_452 = 60.0 as mmmfloat;
        reg_453 = reg_451 * reg_452;
        self.get_current_statestorage().push_pos(1usize);
        reg_454 = 27u64;
        let call_result = self.osc(reg_453);
        reg_455 = call_result;
        reg_456 = 60.0 as mmmfloat;
        reg_457 = reg_455 / reg_456;
        reg_458 = 50.0 as mmmfloat;
        reg_459 = 59.0 as mmmfloat;
        reg_460 = reg_458 * reg_459;
        self.get_current_statestorage().push_pos(1usize);
        reg_461 = 27u64;
        let call_result = self.osc(reg_460);
        reg_462 = call_result;
        reg_463 = 59.0 as mmmfloat;
        reg_464 = reg_462 / reg_463;
        reg_465 = 50.0 as mmmfloat;
        reg_466 = 58.0 as mmmfloat;
        reg_467 = reg_465 * reg_466;
        self.get_current_statestorage().push_pos(1usize);
        reg_468 = 27u64;
        let call_result = self.osc(reg_467);
        reg_469 = call_result;
        reg_470 = 58.0 as mmmfloat;
        reg_471 = reg_469 / reg_470;
        reg_472 = 50.0 as mmmfloat;
        reg_473 = 57.0 as mmmfloat;
        reg_474 = reg_472 * reg_473;
        self.get_current_statestorage().push_pos(1usize);
        reg_475 = 27u64;
        let call_result = self.osc(reg_474);
        reg_476 = call_result;
        reg_477 = 57.0 as mmmfloat;
        reg_478 = reg_476 / reg_477;
        reg_479 = 50.0 as mmmfloat;
        reg_480 = 56.0 as mmmfloat;
        reg_481 = reg_479 * reg_480;
        self.get_current_statestorage().push_pos(1usize);
        reg_482 = 27u64;
        let call_result = self.osc(reg_481);
        reg_483 = call_result;
        reg_484 = 56.0 as mmmfloat;
        reg_485 = reg_483 / reg_484;
        reg_486 = 50.0 as mmmfloat;
        reg_487 = 55.0 as mmmfloat;
        reg_488 = reg_486 * reg_487;
        self.get_current_statestorage().push_pos(1usize);
        reg_489 = 27u64;
        let call_result = self.osc(reg_488);
        reg_490 = call_result;
        reg_491 = 55.0 as mmmfloat;
        reg_492 = reg_490 / reg_491;
        reg_493 = 50.0 as mmmfloat;
        reg_494 = 54.0 as mmmfloat;
        reg_495 = reg_493 * reg_494;
        self.get_current_statestorage().push_pos(1usize);
        reg_496 = 27u64;
        let call_result = self.osc(reg_495);
        reg_497 = call_result;
        reg_498 = 54.0 as mmmfloat;
        reg_499 = reg_497 / reg_498;
        reg_500 = 50.0 as mmmfloat;
        reg_501 = 53.0 as mmmfloat;
        reg_502 = reg_500 * reg_501;
        self.get_current_statestorage().push_pos(1usize);
        reg_503 = 27u64;
        let call_result = self.osc(reg_502);
        reg_504 = call_result;
        reg_505 = 53.0 as mmmfloat;
        reg_506 = reg_504 / reg_505;
        reg_507 = 50.0 as mmmfloat;
        reg_508 = 52.0 as mmmfloat;
        reg_509 = reg_507 * reg_508;
        self.get_current_statestorage().push_pos(1usize);
        reg_510 = 27u64;
        let call_result = self.osc(reg_509);
        reg_511 = call_result;
        reg_512 = 52.0 as mmmfloat;
        reg_513 = reg_511 / reg_512;
        reg_514 = 50.0 as mmmfloat;
        reg_515 = 51.0 as mmmfloat;
        reg_516 = reg_514 * reg_515;
        self.get_current_statestorage().push_pos(1usize);
        reg_517 = 27u64;
        let call_result = self.osc(reg_516);
        reg_518 = call_result;
        reg_519 = 51.0 as mmmfloat;
        reg_520 = reg_518 / reg_519;
        reg_521 = 50.0 as mmmfloat;
        reg_522 = 50.0 as mmmfloat;
        reg_523 = reg_521 * reg_522;
        self.get_current_statestorage().push_pos(1usize);
        reg_524 = 27u64;
        let call_result = self.osc(reg_523);
        reg_525 = call_result;
        reg_526 = 50.0 as mmmfloat;
        reg_527 = reg_525 / reg_526;
        reg_528 = 50.0 as mmmfloat;
        reg_529 = 49.0 as mmmfloat;
        reg_530 = reg_528 * reg_529;
        self.get_current_statestorage().push_pos(1usize);
        reg_531 = 27u64;
        let call_result = self.osc(reg_530);
        reg_532 = call_result;
        reg_533 = 49.0 as mmmfloat;
        reg_534 = reg_532 / reg_533;
        reg_535 = 50.0 as mmmfloat;
        reg_536 = 48.0 as mmmfloat;
        reg_537 = reg_535 * reg_536;
        self.get_current_statestorage().push_pos(1usize);
        reg_538 = 27u64;
        let call_result = self.osc(reg_537);
        reg_539 = call_result;
        reg_540 = 48.0 as mmmfloat;
        reg_541 = reg_539 / reg_540;
        reg_542 = 50.0 as mmmfloat;
        reg_543 = 47.0 as mmmfloat;
        reg_544 = reg_542 * reg_543;
        self.get_current_statestorage().push_pos(1usize);
        reg_545 = 27u64;
        let call_result = self.osc(reg_544);
        reg_546 = call_result;
        reg_547 = 47.0 as mmmfloat;
        reg_548 = reg_546 / reg_547;
        reg_549 = 50.0 as mmmfloat;
        reg_550 = 46.0 as mmmfloat;
        reg_551 = reg_549 * reg_550;
        self.get_current_statestorage().push_pos(1usize);
        reg_552 = 27u64;
        let call_result = self.osc(reg_551);
        reg_553 = call_result;
        reg_554 = 46.0 as mmmfloat;
        reg_555 = reg_553 / reg_554;
        reg_556 = 50.0 as mmmfloat;
        reg_557 = 45.0 as mmmfloat;
        reg_558 = reg_556 * reg_557;
        self.get_current_statestorage().push_pos(1usize);
        reg_559 = 27u64;
        let call_result = self.osc(reg_558);
        reg_560 = call_result;
        reg_561 = 45.0 as mmmfloat;
        reg_562 = reg_560 / reg_561;
        reg_563 = 50.0 as mmmfloat;
        reg_564 = 44.0 as mmmfloat;
        reg_565 = reg_563 * reg_564;
        self.get_current_statestorage().push_pos(1usize);
        reg_566 = 27u64;
        let call_result = self.osc(reg_565);
        reg_567 = call_result;
        reg_568 = 44.0 as mmmfloat;
        reg_569 = reg_567 / reg_568;
        reg_570 = 50.0 as mmmfloat;
        reg_571 = 43.0 as mmmfloat;
        reg_572 = reg_570 * reg_571;
        self.get_current_statestorage().push_pos(1usize);
        reg_573 = 27u64;
        let call_result = self.osc(reg_572);
        reg_574 = call_result;
        reg_575 = 43.0 as mmmfloat;
        reg_576 = reg_574 / reg_575;
        reg_577 = 50.0 as mmmfloat;
        reg_578 = 42.0 as mmmfloat;
        reg_579 = reg_577 * reg_578;
        self.get_current_statestorage().push_pos(1usize);
        reg_580 = 27u64;
        let call_result = self.osc(reg_579);
        reg_581 = call_result;
        reg_582 = 42.0 as mmmfloat;
        reg_583 = reg_581 / reg_582;
        reg_584 = 50.0 as mmmfloat;
        reg_585 = 41.0 as mmmfloat;
        reg_586 = reg_584 * reg_585;
        self.get_current_statestorage().push_pos(1usize);
        reg_587 = 27u64;
        let call_result = self.osc(reg_586);
        reg_588 = call_result;
        reg_589 = 41.0 as mmmfloat;
        reg_590 = reg_588 / reg_589;
        reg_591 = 50.0 as mmmfloat;
        reg_592 = 40.0 as mmmfloat;
        reg_593 = reg_591 * reg_592;
        self.get_current_statestorage().push_pos(1usize);
        reg_594 = 27u64;
        let call_result = self.osc(reg_593);
        reg_595 = call_result;
        reg_596 = 40.0 as mmmfloat;
        reg_597 = reg_595 / reg_596;
        reg_598 = 50.0 as mmmfloat;
        reg_599 = 39.0 as mmmfloat;
        reg_600 = reg_598 * reg_599;
        self.get_current_statestorage().push_pos(1usize);
        reg_601 = 27u64;
        let call_result = self.osc(reg_600);
        reg_602 = call_result;
        reg_603 = 39.0 as mmmfloat;
        reg_604 = reg_602 / reg_603;
        reg_605 = 50.0 as mmmfloat;
        reg_606 = 38.0 as mmmfloat;
        reg_607 = reg_605 * reg_606;
        self.get_current_statestorage().push_pos(1usize);
        reg_608 = 27u64;
        let call_result = self.osc(reg_607);
        reg_609 = call_result;
        reg_610 = 38.0 as mmmfloat;
        reg_611 = reg_609 / reg_610;
        reg_612 = 50.0 as mmmfloat;
        reg_613 = 37.0 as mmmfloat;
        reg_614 = reg_612 * reg_613;
        self.get_current_statestorage().push_pos(1usize);
        reg_615 = 27u64;
        let call_result = self.osc(reg_614);
        reg_616 = call_result;
        reg_617 = 37.0 as mmmfloat;
        reg_618 = reg_616 / reg_617;
        reg_619 = 50.0 as mmmfloat;
        reg_620 = 36.0 as mmmfloat;
        reg_621 = reg_619 * reg_620;
        self.get_current_statestorage().push_pos(1usize);
        reg_622 = 27u64;
        let call_result = self.osc(reg_621);
        reg_623 = call_result;
        reg_624 = 36.0 as mmmfloat;
        reg_625 = reg_623 / reg_624;
        reg_626 = 50.0 as mmmfloat;
        reg_627 = 35.0 as mmmfloat;
        reg_628 = reg_626 * reg_627;
        self.get_current_statestorage().push_pos(1usize);
        reg_629 = 27u64;
        let call_result = self.osc(reg_628);
        reg_630 = call_result;
        reg_631 = 35.0 as mmmfloat;
        reg_632 = reg_630 / reg_631;
        reg_633 = 50.0 as mmmfloat;
        reg_634 = 34.0 as mmmfloat;
        reg_635 = reg_633 * reg_634;
        self.get_current_statestorage().push_pos(1usize);
        reg_636 = 27u64;
        let call_result = self.osc(reg_635);
        reg_637 = call_result;
        reg_638 = 34.0 as mmmfloat;
        reg_639 = reg_637 / reg_638;
        reg_640 = 50.0 as mmmfloat;
        reg_641 = 33.0 as mmmfloat;
        reg_642 = reg_640 * reg_641;
        self.get_current_statestorage().push_pos(1usize);
        reg_643 = 27u64;
        let call_result = self.osc(reg_642);
        reg_644 = call_result;
        reg_645 = 33.0 as mmmfloat;
        reg_646 = reg_644 / reg_645;
        reg_647 = 50.0 as mmmfloat;
        reg_648 = 32.0 as mmmfloat;
        reg_649 = reg_647 * reg_648;
        self.get_current_statestorage().push_pos(1usize);
        reg_650 = 27u64;
        let call_result = self.osc(reg_649);
        reg_651 = call_result;
        reg_652 = 32.0 as mmmfloat;
        reg_653 = reg_651 / reg_652;
        reg_654 = 50.0 as mmmfloat;
        reg_655 = 31.0 as mmmfloat;
        reg_656 = reg_654 * reg_655;
        self.get_current_statestorage().push_pos(1usize);
        reg_657 = 27u64;
        let call_result = self.osc(reg_656);
        reg_658 = call_result;
        reg_659 = 31.0 as mmmfloat;
        reg_660 = reg_658 / reg_659;
        reg_661 = 50.0 as mmmfloat;
        reg_662 = 30.0 as mmmfloat;
        reg_663 = reg_661 * reg_662;
        self.get_current_statestorage().push_pos(1usize);
        reg_664 = 27u64;
        let call_result = self.osc(reg_663);
        reg_665 = call_result;
        reg_666 = 30.0 as mmmfloat;
        reg_667 = reg_665 / reg_666;
        reg_668 = 50.0 as mmmfloat;
        reg_669 = 29.0 as mmmfloat;
        reg_670 = reg_668 * reg_669;
        self.get_current_statestorage().push_pos(1usize);
        reg_671 = 27u64;
        let call_result = self.osc(reg_670);
        reg_672 = call_result;
        reg_673 = 29.0 as mmmfloat;
        reg_674 = reg_672 / reg_673;
        reg_675 = 50.0 as mmmfloat;
        reg_676 = 28.0 as mmmfloat;
        reg_677 = reg_675 * reg_676;
        self.get_current_statestorage().push_pos(1usize);
        reg_678 = 27u64;
        let call_result = self.osc(reg_677);
        reg_679 = call_result;
        reg_680 = 28.0 as mmmfloat;
        reg_681 = reg_679 / reg_680;
        reg_682 = 50.0 as mmmfloat;
        reg_683 = 27.0 as mmmfloat;
        reg_684 = reg_682 * reg_683;
        self.get_current_statestorage().push_pos(1usize);
        reg_685 = 27u64;
        let call_result = self.osc(reg_684);
        reg_686 = call_result;
        reg_687 = 27.0 as mmmfloat;
        reg_688 = reg_686 / reg_687;
        reg_689 = 50.0 as mmmfloat;
        reg_690 = 26.0 as mmmfloat;
        reg_691 = reg_689 * reg_690;
        self.get_current_statestorage().push_pos(1usize);
        reg_692 = 27u64;
        let call_result = self.osc(reg_691);
        reg_693 = call_result;
        reg_694 = 26.0 as mmmfloat;
        reg_695 = reg_693 / reg_694;
        reg_696 = 50.0 as mmmfloat;
        reg_697 = 25.0 as mmmfloat;
        reg_698 = reg_696 * reg_697;
        self.get_current_statestorage().push_pos(1usize);
        reg_699 = 27u64;
        let call_result = self.osc(reg_698);
        reg_700 = call_result;
        reg_701 = 25.0 as mmmfloat;
        reg_702 = reg_700 / reg_701;
        reg_703 = 50.0 as mmmfloat;
        reg_704 = 24.0 as mmmfloat;
        reg_705 = reg_703 * reg_704;
        self.get_current_statestorage().push_pos(1usize);
        reg_706 = 27u64;
        let call_result = self.osc(reg_705);
        reg_707 = call_result;
        reg_708 = 24.0 as mmmfloat;
        reg_709 = reg_707 / reg_708;
        reg_710 = 50.0 as mmmfloat;
        reg_711 = 23.0 as mmmfloat;
        reg_712 = reg_710 * reg_711;
        self.get_current_statestorage().push_pos(1usize);
        reg_713 = 27u64;
        let call_result = self.osc(reg_712);
        reg_714 = call_result;
        reg_715 = 23.0 as mmmfloat;
        reg_716 = reg_714 / reg_715;
        reg_717 = 50.0 as mmmfloat;
        reg_718 = 22.0 as mmmfloat;
        reg_719 = reg_717 * reg_718;
        self.get_current_statestorage().push_pos(1usize);
        reg_720 = 27u64;
        let call_result = self.osc(reg_719);
        reg_721 = call_result;
        reg_722 = 22.0 as mmmfloat;
        reg_723 = reg_721 / reg_722;
        reg_724 = 50.0 as mmmfloat;
        reg_725 = 21.0 as mmmfloat;
        reg_726 = reg_724 * reg_725;
        self.get_current_statestorage().push_pos(1usize);
        reg_727 = 27u64;
        let call_result = self.osc(reg_726);
        reg_728 = call_result;
        reg_729 = 21.0 as mmmfloat;
        reg_730 = reg_728 / reg_729;
        reg_731 = 50.0 as mmmfloat;
        reg_732 = 20.0 as mmmfloat;
        reg_733 = reg_731 * reg_732;
        self.get_current_statestorage().push_pos(1usize);
        reg_734 = 27u64;
        let call_result = self.osc(reg_733);
        reg_735 = call_result;
        reg_736 = 20.0 as mmmfloat;
        reg_737 = reg_735 / reg_736;
        reg_738 = 50.0 as mmmfloat;
        reg_739 = 19.0 as mmmfloat;
        reg_740 = reg_738 * reg_739;
        self.get_current_statestorage().push_pos(1usize);
        reg_741 = 27u64;
        let call_result = self.osc(reg_740);
        reg_742 = call_result;
        reg_743 = 19.0 as mmmfloat;
        reg_744 = reg_742 / reg_743;
        reg_745 = 50.0 as mmmfloat;
        reg_746 = 18.0 as mmmfloat;
        reg_747 = reg_745 * reg_746;
        self.get_current_statestorage().push_pos(1usize);
        reg_748 = 27u64;
        let call_result = self.osc(reg_747);
        reg_749 = call_result;
        reg_750 = 18.0 as mmmfloat;
        reg_751 = reg_749 / reg_750;
        reg_752 = 50.0 as mmmfloat;
        reg_753 = 17.0 as mmmfloat;
        reg_754 = reg_752 * reg_753;
        self.get_current_statestorage().push_pos(1usize);
        reg_755 = 27u64;
        let call_result = self.osc(reg_754);
        reg_756 = call_result;
        reg_757 = 17.0 as mmmfloat;
        reg_758 = reg_756 / reg_757;
        reg_759 = 50.0 as mmmfloat;
        reg_760 = 16.0 as mmmfloat;
        reg_761 = reg_759 * reg_760;
        self.get_current_statestorage().push_pos(1usize);
        reg_762 = 27u64;
        let call_result = self.osc(reg_761);
        reg_763 = call_result;
        reg_764 = 16.0 as mmmfloat;
        reg_765 = reg_763 / reg_764;
        reg_766 = 50.0 as mmmfloat;
        reg_767 = 15.0 as mmmfloat;
        reg_768 = reg_766 * reg_767;
        self.get_current_statestorage().push_pos(1usize);
        reg_769 = 27u64;
        let call_result = self.osc(reg_768);
        reg_770 = call_result;
        reg_771 = 15.0 as mmmfloat;
        reg_772 = reg_770 / reg_771;
        reg_773 = 50.0 as mmmfloat;
        reg_774 = 14.0 as mmmfloat;
        reg_775 = reg_773 * reg_774;
        self.get_current_statestorage().push_pos(1usize);
        reg_776 = 27u64;
        let call_result = self.osc(reg_775);
        reg_777 = call_result;
        reg_778 = 14.0 as mmmfloat;
        reg_779 = reg_777 / reg_778;
        reg_780 = 50.0 as mmmfloat;
        reg_781 = 13.0 as mmmfloat;
        reg_782 = reg_780 * reg_781;
        self.get_current_statestorage().push_pos(1usize);
        reg_783 = 27u64;
        let call_result = self.osc(reg_782);
        reg_784 = call_result;
        reg_785 = 13.0 as mmmfloat;
        reg_786 = reg_784 / reg_785;
        reg_787 = 50.0 as mmmfloat;
        reg_788 = 12.0 as mmmfloat;
        reg_789 = reg_787 * reg_788;
        self.get_current_statestorage().push_pos(1usize);
        reg_790 = 27u64;
        let call_result = self.osc(reg_789);
        reg_791 = call_result;
        reg_792 = 12.0 as mmmfloat;
        reg_793 = reg_791 / reg_792;
        reg_794 = 50.0 as mmmfloat;
        reg_795 = 11.0 as mmmfloat;
        reg_796 = reg_794 * reg_795;
        self.get_current_statestorage().push_pos(1usize);
        reg_797 = 27u64;
        let call_result = self.osc(reg_796);
        reg_798 = call_result;
        reg_799 = 11.0 as mmmfloat;
        reg_800 = reg_798 / reg_799;
        reg_801 = 50.0 as mmmfloat;
        reg_802 = 10.0 as mmmfloat;
        reg_803 = reg_801 * reg_802;
        self.get_current_statestorage().push_pos(1usize);
        reg_804 = 27u64;
        let call_result = self.osc(reg_803);
        reg_805 = call_result;
        reg_806 = 10.0 as mmmfloat;
        reg_807 = reg_805 / reg_806;
        reg_808 = 50.0 as mmmfloat;
        reg_809 = 9.0 as mmmfloat;
        reg_810 = reg_808 * reg_809;
        self.get_current_statestorage().push_pos(1usize);
        reg_811 = 27u64;
        let call_result = self.osc(reg_810);
        reg_812 = call_result;
        reg_813 = 9.0 as mmmfloat;
        reg_814 = reg_812 / reg_813;
        reg_815 = 50.0 as mmmfloat;
        reg_816 = 8.0 as mmmfloat;
        reg_817 = reg_815 * reg_816;
        self.get_current_statestorage().push_pos(1usize);
        reg_818 = 27u64;
        let call_result = self.osc(reg_817);
        reg_819 = call_result;
        reg_820 = 8.0 as mmmfloat;
        reg_821 = reg_819 / reg_820;
        reg_822 = 50.0 as mmmfloat;
        reg_823 = 7.0 as mmmfloat;
        reg_824 = reg_822 * reg_823;
        self.get_current_statestorage().push_pos(1usize);
        reg_825 = 27u64;
        let call_result = self.osc(reg_824);
        reg_826 = call_result;
        reg_827 = 7.0 as mmmfloat;
        reg_828 = reg_826 / reg_827;
        reg_829 = 50.0 as mmmfloat;
        reg_830 = 6.0 as mmmfloat;
        reg_831 = reg_829 * reg_830;
        self.get_current_statestorage().push_pos(1usize);
        reg_832 = 27u64;
        let call_result = self.osc(reg_831);
        reg_833 = call_result;
        reg_834 = 6.0 as mmmfloat;
        reg_835 = reg_833 / reg_834;
        reg_836 = 50.0 as mmmfloat;
        reg_837 = 5.0 as mmmfloat;
        reg_838 = reg_836 * reg_837;
        self.get_current_statestorage().push_pos(1usize);
        reg_839 = 27u64;
        let call_result = self.osc(reg_838);
        reg_840 = call_result;
        reg_841 = 5.0 as mmmfloat;
        reg_842 = reg_840 / reg_841;
        reg_843 = 50.0 as mmmfloat;
        reg_844 = 4.0 as mmmfloat;
        reg_845 = reg_843 * reg_844;
        self.get_current_statestorage().push_pos(1usize);
        reg_846 = 27u64;
        let call_result = self.osc(reg_845);
        reg_847 = call_result;
        reg_848 = 4.0 as mmmfloat;
        reg_849 = reg_847 / reg_848;
        reg_850 = 50.0 as mmmfloat;
        reg_851 = 3.0 as mmmfloat;
        reg_852 = reg_850 * reg_851;
        self.get_current_statestorage().push_pos(1usize);
        reg_853 = 27u64;
        let call_result = self.osc(reg_852);
        reg_854 = call_result;
        reg_855 = 3.0 as mmmfloat;
        reg_856 = reg_854 / reg_855;
        reg_857 = 50.0 as mmmfloat;
        reg_858 = 2.0 as mmmfloat;
        reg_859 = reg_857 * reg_858;
        self.get_current_statestorage().push_pos(1usize);
        reg_860 = 27u64;
        let call_result = self.osc(reg_859);
        reg_861 = call_result;
        reg_862 = 2.0 as mmmfloat;
        reg_863 = reg_861 / reg_862;
        reg_864 = 50.0 as mmmfloat;
        reg_865 = 1.0 as mmmfloat;
        reg_866 = reg_864 * reg_865;
        self.get_current_statestorage().push_pos(1usize);
        reg_867 = 27u64;
        let call_result = self.osc(reg_866);
        reg_868 = call_result;
        reg_869 = 1.0 as mmmfloat;
        reg_870 = reg_868 / reg_869;
        reg_871 = 50.0 as mmmfloat;
        self.get_current_statestorage().push_pos(1usize);
        reg_872 = 27u64;
        let call_result = self.osc(reg_871);
        reg_873 = call_result;
        reg_874 = reg_870 + reg_873;
        reg_875 = reg_863 + reg_874;
        reg_876 = reg_856 + reg_875;
        reg_877 = reg_849 + reg_876;
        reg_878 = reg_842 + reg_877;
        reg_879 = reg_835 + reg_878;
        reg_880 = reg_828 + reg_879;
        reg_881 = reg_821 + reg_880;
        reg_882 = reg_814 + reg_881;
        reg_883 = reg_807 + reg_882;
        reg_884 = reg_800 + reg_883;
        reg_885 = reg_793 + reg_884;
        reg_886 = reg_786 + reg_885;
        reg_887 = reg_779 + reg_886;
        reg_888 = reg_772 + reg_887;
        reg_889 = reg_765 + reg_888;
        reg_890 = reg_758 + reg_889;
        reg_891 = reg_751 + reg_890;
        reg_892 = reg_744 + reg_891;
        reg_893 = reg_737 + reg_892;
        reg_894 = reg_730 + reg_893;
        reg_895 = reg_723 + reg_894;
        reg_896 = reg_716 + reg_895;
        reg_897 = reg_709 + reg_896;
        reg_898 = reg_702 + reg_897;
        reg_899 = reg_695 + reg_898;
        reg_900 = reg_688 + reg_899;
        reg_901 = reg_681 + reg_900;
        reg_902 = reg_674 + reg_901;
        reg_903 = reg_667 + reg_902;
        reg_904 = reg_660 + reg_903;
        reg_905 = reg_653 + reg_904;
        reg_906 = reg_646 + reg_905;
        reg_907 = reg_639 + reg_906;
        reg_908 = reg_632 + reg_907;
        reg_909 = reg_625 + reg_908;
        reg_910 = reg_618 + reg_909;
        reg_911 = reg_611 + reg_910;
        reg_912 = reg_604 + reg_911;
        reg_913 = reg_597 + reg_912;
        reg_914 = reg_590 + reg_913;
        reg_915 = reg_583 + reg_914;
        reg_916 = reg_576 + reg_915;
        reg_917 = reg_569 + reg_916;
        reg_918 = reg_562 + reg_917;
        reg_919 = reg_555 + reg_918;
        reg_920 = reg_548 + reg_919;
        reg_921 = reg_541 + reg_920;
        reg_922 = reg_534 + reg_921;
        reg_923 = reg_527 + reg_922;
        reg_924 = reg_520 + reg_923;
        reg_925 = reg_513 + reg_924;
        reg_926 = reg_506 + reg_925;
        reg_927 = reg_499 + reg_926;
        reg_928 = reg_492 + reg_927;
        reg_929 = reg_485 + reg_928;
        reg_930 = reg_478 + reg_929;
        reg_931 = reg_471 + reg_930;
        reg_932 = reg_464 + reg_931;
        reg_933 = reg_457 + reg_932;
        reg_934 = reg_450 + reg_933;
        reg_935 = reg_443 + reg_934;
        reg_936 = reg_436 + reg_935;
        reg_937 = reg_429 + reg_936;
        reg_938 = reg_422 + reg_937;
        reg_939 = reg_415 + reg_938;
        reg_940 = reg_408 + reg_939;
        reg_941 = reg_401 + reg_940;
        reg_942 = reg_394 + reg_941;
        reg_943 = reg_387 + reg_942;
        reg_944 = reg_380 + reg_943;
        reg_945 = reg_373 + reg_944;
        reg_946 = reg_366 + reg_945;
        reg_947 = reg_359 + reg_946;
        reg_948 = reg_352 + reg_947;
        reg_949 = reg_345 + reg_948;
        reg_950 = reg_338 + reg_949;
        reg_951 = reg_331 + reg_950;
        reg_952 = reg_324 + reg_951;
        reg_953 = reg_317 + reg_952;
        reg_954 = reg_310 + reg_953;
        reg_955 = reg_303 + reg_954;
        reg_956 = reg_296 + reg_955;
        reg_957 = reg_289 + reg_956;
        reg_958 = reg_282 + reg_957;
        reg_959 = reg_275 + reg_958;
        reg_960 = reg_268 + reg_959;
        reg_961 = reg_261 + reg_960;
        reg_962 = reg_254 + reg_961;
        reg_963 = reg_247 + reg_962;
        reg_964 = reg_240 + reg_963;
        reg_965 = reg_233 + reg_964;
        reg_966 = reg_226 + reg_965;
        reg_967 = reg_219 + reg_966;
        reg_968 = reg_212 + reg_967;
        reg_969 = reg_205 + reg_968;
        reg_970 = reg_198 + reg_969;
        reg_971 = reg_191 + reg_970;
        reg_972 = reg_184 + reg_971;
        reg_973 = reg_177 + reg_972;
        stack_alloc_974[0usize] = f64_to_word(reg_973);
        reg_977 = word_to_f64(stack_alloc_974[0usize]);
        stack_alloc_976[0usize] = f64_to_word(reg_977);
        reg_980 = word_to_f64(stack_alloc_974[0usize]);
        stack_alloc_976[1usize] = f64_to_word(reg_980);
        self.get_current_statestorage().pop_pos(100usize);
        ret_words.copy_from_slice(&stack_alloc_976[0usize..2usize]);
        return ();
    }

}
