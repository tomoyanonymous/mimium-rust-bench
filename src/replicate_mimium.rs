use std::convert::TryInto;

pub type Word = u64;

#[inline(always)]
fn f64_to_word(value: f64) -> Word { value.to_bits() }
#[inline(always)]
fn word_to_f64(value: Word) -> f64 { f64::from_bits(value) }
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

    fn current_time(&mut self) -> f64 {
        0.0
    }

    fn sample_rate(&mut self) -> f64 {
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

    fn push_pos(&mut self, offset: usize) {
        self.pos = self.pos.saturating_add(offset);
    }

    fn pop_pos(&mut self, offset: usize) {
        self.pos = self.pos.saturating_sub(offset);
    }

    fn get_state(&mut self, size: usize) -> Vec<Word> {
        self.ensure(size);
        self.rawdata[self.pos..self.pos + size].to_vec()
    }

    fn set_state(&mut self, src: &[Word], size: usize) {
        self.ensure(size);
        self.rawdata[self.pos..self.pos + size].copy_from_slice(&src[..size]);
    }

    fn mem(&mut self, src: Word) -> Word {
        self.ensure(1);
        let prev = self.rawdata[self.pos];
        self.rawdata[self.pos] = src;
        prev
    }

    fn delay(&mut self, input: Word, time_raw: Word, max_len: usize) -> Word {
        let total_words = max_len.saturating_add(2);
        self.ensure(total_words);
        if max_len == 0 {
            return 0;
        }

        let delay_samples = word_to_f64(time_raw)
            .clamp(0.0, max_len.saturating_sub(1) as f64) as usize;
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
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),
                StateStorage::new(1),

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
        let final_result = if result.is_empty() { Ok(Vec::new()) } else { self.memory.load(result[0], 2usize) };
        final_result
    }

    pub fn call_dsp_buffer(&mut self, input: &[f64], output: &mut [f64], frames: usize) -> Result<(), String> {
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
            let result = self.dsp();
            output[frame_output_start + 0usize] = word_to_f64(result.0);
            output[frame_output_start + 1usize] = word_to_f64(result.1);
        }
        self.current_function_state = previous_function_state;
        Ok(())
    }



    fn call_function_handle(&mut self, handle: Word, args: &[Word]) -> Vec<Word> {
        self.call_function_handle_with_memory(handle, args)
    }

    fn get_current_statestorage(&mut self) -> &mut StateStorage {
        if let Some(&closure_handle) = self.state_storage_stack.last() {
            &mut self
                .closures
                .get_mut(closure_handle)
                .unwrap_or_else(|err| unreachable!("{err}"))
                .state_storage
        } else {
            let function_index = self
                .current_function_state
                .unwrap_or_else(|| unreachable!("missing active function state"));
            &mut self.function_states[function_index]
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
                    Ok(vec![f64_to_word(array.data.len() as f64)])
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
            Some(29) => {
                let result = self.dispatch_r(args);
                result
            },
            Some(30) => {
                let result = self.dispatch_lambda_0(args);
                result
            },
            Some(31) => {
                let result = self.dispatch_lambda_1(args);
                result
            },
            Some(32) => {
                let result = self.dispatch_lambda_2(args);
                result
            },
            Some(33) => {
                let result = self.dispatch_lambda_3(args);
                result
            },
            Some(34) => {
                let result = self.dispatch_lambda_4(args);
                result
            },
            Some(35) => {
                let result = self.dispatch_lambda_5(args);
                result
            },
            Some(36) => {
                let result = self.dispatch_lambda_6(args);
                result
            },
            Some(37) => {
                let result = self.dispatch_lambda_7(args);
                result
            },
            Some(38) => {
                let result = self.dispatch_lambda_8(args);
                result
            },
            Some(39) => {
                let result = self.dispatch_lambda_9(args);
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

    fn dispatch__mimium_global(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self._mimium_global();
        Vec::new()
    }

    #[inline(always)]
    fn _mimium_global(&mut self) -> () {
        let mut reg_321 = [0u64; 0];
        return ();
    }

    fn dispatch_math_PI(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.math_PI();
        [result].to_vec()
    }

    #[inline(always)]
    fn math_PI(&mut self) -> Word {
        let mut reg_0 = [0u64; 1];
        let mut reg_1 = [0u64; 1];
        reg_0[0] = f64_to_word(3.14159265359f64);
        return reg_0[0];
    }

    fn dispatch_math_E(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.math_E();
        [result].to_vec()
    }

    #[inline(always)]
    fn math_E(&mut self) -> Word {
        let mut reg_2 = [0u64; 1];
        let mut reg_3 = [0u64; 1];
        reg_2[0] = f64_to_word(2.71828182846f64);
        return reg_2[0];
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
        let mut reg_4 = [0u64; 1];
        let mut reg_5 = [0u64; 1];
        let mut reg_6 = [0u64; 1];
        let mut reg_7 = [0u64; 1];
        let mut reg_8 = [0u64; 1];
        reg_4[0] = 2u64;
        let call_result = self.math_E();
        reg_5 = [call_result];
        reg_6 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_7[0] = f64_to_word(word_to_f64(reg_5[0]).powf(word_to_f64(reg_6[0])));
        return reg_7[0];
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
        let mut reg_9 = [0u64; 1];
        let mut reg_10 = [0u64; 1];
        let mut reg_11 = [0u64; 1];
        let mut reg_12 = [0u64; 1];
        let mut reg_13 = [0u64; 1];
        let mut reg_14 = [0u64; 1];
        reg_9 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_10[0] = f64_to_word(word_to_f64(reg_9[0]).ln());
        reg_11[0] = f64_to_word(2.0f64);
        reg_12[0] = f64_to_word(word_to_f64(reg_11[0]).ln());
        reg_13[0] = f64_to_word(word_to_f64(reg_10[0]) / word_to_f64(reg_12[0]));
        return reg_13[0];
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
        let mut reg_15 = [0u64; 1];
        let mut reg_16 = [0u64; 1];
        let mut reg_17 = [0u64; 1];
        let mut reg_18 = [0u64; 1];
        let mut reg_19 = [0u64; 1];
        let mut reg_20 = [0u64; 1];
        reg_15 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_16[0] = f64_to_word(word_to_f64(reg_15[0]).ln());
        reg_17[0] = f64_to_word(10.0f64);
        reg_18[0] = f64_to_word(word_to_f64(reg_17[0]).ln());
        reg_19[0] = f64_to_word(word_to_f64(reg_16[0]) / word_to_f64(reg_18[0]));
        return reg_19[0];
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
        let mut reg_21 = [0u64; 1];
        let mut reg_22 = [0u64; 1];
        let mut reg_23 = [0u64; 1];
        let mut reg_24 = [0u64; 1];
        let mut reg_25 = [0u64; 1];
        let mut reg_26 = [0u64; 1];
        let mut reg_27 = [0u64; 1];
        let mut reg_28 = [0u64; 1];
        let mut reg_29 = [0u64; 1];
        reg_21 = vec_to_words::<1>({ let state = self.get_current_statestorage(); state.get_state(1usize) }).unwrap();
        reg_22 = vec_to_words::<1>(self.memory.load(reg_21[0], 1usize).unwrap()).unwrap();
        reg_23 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_24[0] = f64_to_word(self.host.sample_rate());
        reg_25[0] = f64_to_word(word_to_f64(reg_23[0]) / word_to_f64(reg_24[0]));
        reg_26[0] = f64_to_word(word_to_f64(reg_22[0]) + word_to_f64(reg_25[0]));
        reg_27[0] = f64_to_word(1.0f64);
        reg_28[0] = f64_to_word(word_to_f64(reg_26[0]) % word_to_f64(reg_27[0]));
        let result = reg_28[0];
        let next_state_words = &[reg_28[0]].to_vec();
        {
            let state = self.get_current_statestorage();
            state.set_state(&next_state_words, 1usize);
        }
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
        let arg_1 = [arg_1_value];
        let mut reg_32 = [0u64; 1];
        let mut reg_33 = [0u64; 1];
        let mut reg_34 = [0u64; 1];
        let mut reg_35 = [0u64; 1];
        let mut reg_36 = [0u64; 1];
        let mut reg_37 = [0u64; 1];
        let mut reg_38 = [0u64; 1];
        let mut reg_39 = [0u64; 1];
        reg_32 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_33[0] = 6u64;
        let call_result = self.osc_phasor_zero(reg_32[0]);
        reg_34 = [call_result];
        reg_35 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
        reg_36[0] = f64_to_word(word_to_f64(reg_34[0]) + word_to_f64(reg_35[0]));
        reg_37[0] = f64_to_word(1.0f64);
        reg_38[0] = f64_to_word(word_to_f64(reg_36[0]) % word_to_f64(reg_37[0]));
        return reg_38[0];
    }

    fn dispatch___default_7_phase_shift(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_7_phase_shift();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_7_phase_shift(&mut self) -> Word {
        let mut reg_30 = [0u64; 1];
        let mut reg_31 = [0u64; 1];
        reg_30[0] = f64_to_word(0.0f64);
        return reg_30[0];
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
        let arg_1 = [arg_1_value];
        let mut reg_42 = [0u64; 1];
        let mut reg_43 = [0u64; 1];
        let mut reg_44 = [0u64; 1];
        let mut reg_45 = [0u64; 1];
        let mut reg_46 = [0u64; 1];
        let mut reg_47 = [0u64; 1];
        let mut reg_48 = [0u64; 1];
        let mut reg_49 = [0u64; 1];
        let mut reg_50 = [0u64; 1];
        reg_42 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_43 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
        reg_44[0] = 7u64;
        let call_result = self.osc_phasor(reg_42[0], reg_43[0]);
        reg_45 = [call_result];
        reg_46[0] = f64_to_word(2.0f64);
        reg_47[0] = f64_to_word(word_to_f64(reg_45[0]) * word_to_f64(reg_46[0]));
        reg_48[0] = f64_to_word(1.0f64);
        reg_49[0] = f64_to_word(word_to_f64(reg_47[0]) - word_to_f64(reg_48[0]));
        return reg_49[0];
    }

    fn dispatch___default_9_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_9_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_9_phase(&mut self) -> Word {
        let mut reg_40 = [0u64; 1];
        let mut reg_41 = [0u64; 1];
        reg_40[0] = f64_to_word(0.0f64);
        return reg_40[0];
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
        let arg_1 = [arg_1_value];
        let mut reg_53 = [0u64; 1];
        let mut reg_54 = [0u64; 1];
        let mut reg_55 = [0u64; 1];
        let mut reg_56 = [0u64; 1];
        let mut reg_57 = [0u64; 1];
        reg_53 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_54 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
        reg_55[0] = 9u64;
        let call_result = self.osc_lfsaw(reg_53[0], reg_54[0]);
        reg_56 = [call_result];
        return reg_56[0];
    }

    fn dispatch___default_11_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_11_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_11_phase(&mut self) -> Word {
        let mut reg_51 = [0u64; 1];
        let mut reg_52 = [0u64; 1];
        reg_51[0] = f64_to_word(0.0f64);
        return reg_51[0];
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
        let arg_1 = [arg_1_value];
        let mut reg_60 = [0u64; 1];
        let mut reg_61 = [0u64; 1];
        let mut reg_62 = [0u64; 1];
        let mut reg_63 = [0u64; 1];
        let mut reg_64 = [0u64; 1];
        let mut reg_65 = [0u64; 1];
        let mut reg_66 = [0u64; 1];
        let mut reg_67 = [0u64; 1];
        let mut reg_68 = [0u64; 1];
        let mut reg_69 = [0u64; 1];
        let mut reg_70 = [0u64; 1];
        let mut reg_71 = [0u64; 1];
        let mut reg_72 = [0u64; 1];
        let mut reg_73 = [0u64; 1];
        let mut reg_74 = [0u64; 1];
        let mut reg_75 = [0u64; 1];
        let mut reg_76 = [0u64; 1];
        let mut reg_77 = [0u64; 1];
        let mut reg_78 = [0u64; 1];
        let mut reg_79 = [0u64; 1];
        let mut reg_80 = [0u64; 1];
        let mut reg_81 = [0u64; 1];
        let mut reg_82 = [0u64; 1];
        let mut reg_83 = [0u64; 1];
        let mut reg_84 = [0u64; 1];
        let mut reg_85 = [0u64; 1];
        let mut reg_86 = [0u64; 1];
        let mut reg_87 = [0u64; 1];
        let mut reg_88 = [0u64; 1];
        let mut reg_89 = [0u64; 1];
        let mut reg_90 = [0u64; 1];
        let mut reg_91 = [0u64; 1];
        let mut reg_92 = [0u64; 1];
        let mut reg_93 = [0u64; 1];
        let mut reg_94 = [0u64; 1];
        let mut reg_95 = [0u64; 1];
        let mut reg_96 = [0u64; 1];
        let mut reg_97 = [0u64; 1];
        let mut reg_98 = [0u64; 1];
        let mut bb: usize = 0;
        let mut pred_bb: usize = 0;
        loop {
            match bb {
                0 => {
                    reg_60 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
                    reg_61 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
                    reg_62[0] = 7u64;
                    let call_result = self.osc_phasor(reg_60[0], reg_61[0]);
                    reg_63 = [call_result];
                    reg_64[0] = self.memory.alloc(1usize);
                    self.memory.store(reg_64[0], &[reg_63[0]], 1usize).unwrap();
                    reg_66 = vec_to_words::<1>(self.memory.load(reg_64[0], 1usize).unwrap()).unwrap();
                    reg_67[0] = f64_to_word(0.25f64);
                    reg_68[0] = f64_to_word(if word_to_f64(reg_66[0]) < word_to_f64(reg_67[0]) { 1.0 } else { 0.0 });
                    pred_bb = 0;
                    bb = if truthy(reg_68[0]) { 1usize } else { 2usize };
                    continue;
                },
                1 => {
                    reg_70 = vec_to_words::<1>(self.memory.load(reg_64[0], 1usize).unwrap()).unwrap();
                    reg_71[0] = f64_to_word(0.0f64);
                    reg_72[0] = f64_to_word(1.0f64);
                    reg_73[0] = f64_to_word(word_to_f64(reg_71[0]) - word_to_f64(reg_72[0]));
                    reg_74[0] = f64_to_word(word_to_f64(reg_70[0]) * word_to_f64(reg_73[0]));
                    reg_75[0] = f64_to_word(0.5f64);
                    reg_76[0] = f64_to_word(word_to_f64(reg_74[0]) + word_to_f64(reg_75[0]));
                    pred_bb = 1;
                    bb = 6usize;
                    continue;
                },
                2 => {
                    reg_77 = vec_to_words::<1>(self.memory.load(reg_64[0], 1usize).unwrap()).unwrap();
                    reg_78[0] = f64_to_word(0.75f64);
                    reg_79[0] = f64_to_word(if word_to_f64(reg_77[0]) > word_to_f64(reg_78[0]) { 1.0 } else { 0.0 });
                    pred_bb = 2;
                    bb = if truthy(reg_79[0]) { 3usize } else { 4usize };
                    continue;
                    pred_bb = 2;
                    bb = 6usize;
                    continue;
                },
                3 => {
                    reg_81 = vec_to_words::<1>(self.memory.load(reg_64[0], 1usize).unwrap()).unwrap();
                    reg_82[0] = f64_to_word(0.0f64);
                    reg_83[0] = f64_to_word(1.0f64);
                    reg_84[0] = f64_to_word(word_to_f64(reg_82[0]) - word_to_f64(reg_83[0]));
                    reg_85[0] = f64_to_word(word_to_f64(reg_81[0]) * word_to_f64(reg_84[0]));
                    reg_86[0] = f64_to_word(1.5f64);
                    reg_87[0] = f64_to_word(word_to_f64(reg_85[0]) + word_to_f64(reg_86[0]));
                    pred_bb = 3;
                    bb = 5usize;
                    continue;
                },
                4 => {
                    reg_88 = vec_to_words::<1>(self.memory.load(reg_64[0], 1usize).unwrap()).unwrap();
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
                    reg_91[0] = self.memory.alloc(1usize);
                    self.memory.store(reg_91[0], &[reg_90[0]], 1usize).unwrap();
                    reg_93 = vec_to_words::<1>(self.memory.load(reg_91[0], 1usize).unwrap()).unwrap();
                    reg_94[0] = f64_to_word(0.5f64);
                    reg_95[0] = f64_to_word(word_to_f64(reg_93[0]) - word_to_f64(reg_94[0]));
                    reg_96[0] = f64_to_word(4.0f64);
                    reg_97[0] = f64_to_word(word_to_f64(reg_95[0]) * word_to_f64(reg_96[0]));
                    return reg_97[0];
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
        let mut reg_58 = [0u64; 1];
        let mut reg_59 = [0u64; 1];
        reg_58[0] = f64_to_word(0.0f64);
        return reg_58[0];
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
        let arg_1 = [arg_1_value];
        let mut reg_101 = [0u64; 1];
        let mut reg_102 = [0u64; 1];
        let mut reg_103 = [0u64; 1];
        let mut reg_104 = [0u64; 1];
        let mut reg_105 = [0u64; 1];
        let mut reg_106 = [0u64; 1];
        let mut reg_107 = [0u64; 1];
        let mut reg_108 = [0u64; 1];
        let mut reg_109 = [0u64; 1];
        reg_101 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_102 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
        reg_103[0] = 13u64;
        let call_result = self.osc_tri(reg_101[0], reg_102[0]);
        reg_104 = [call_result];
        reg_105[0] = f64_to_word(0.5f64);
        reg_106[0] = f64_to_word(word_to_f64(reg_104[0]) * word_to_f64(reg_105[0]));
        reg_107[0] = f64_to_word(0.5f64);
        reg_108[0] = f64_to_word(word_to_f64(reg_106[0]) + word_to_f64(reg_107[0]));
        return reg_108[0];
    }

    fn dispatch___default_15_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_15_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_15_phase(&mut self) -> Word {
        let mut reg_99 = [0u64; 1];
        let mut reg_100 = [0u64; 1];
        reg_99[0] = f64_to_word(0.0f64);
        return reg_99[0];
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
        let arg_1 = [arg_1_value];
        let arg_2 = [arg_2_value];
        let mut reg_114 = [0u64; 1];
        let mut reg_115 = [0u64; 1];
        let mut reg_116 = [0u64; 1];
        let mut reg_117 = [0u64; 1];
        let mut reg_118 = [0u64; 1];
        let mut reg_119 = [0u64; 1];
        let mut reg_120 = [0u64; 1];
        let mut reg_121 = [0u64; 1];
        let mut reg_122 = [0u64; 1];
        let mut reg_123 = [0u64; 1];
        let mut reg_124 = [0u64; 1];
        let mut reg_125 = [0u64; 1];
        let mut reg_126 = [0u64; 1];
        let mut bb: usize = 0;
        let mut pred_bb: usize = 0;
        loop {
            match bb {
                0 => {
                    reg_114 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
                    reg_115 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
                    reg_116[0] = 7u64;
                    let call_result = self.osc_phasor(reg_114[0], reg_115[0]);
                    reg_117 = [call_result];
                    reg_118 = vec_to_words::<1>(self.memory.load(arg_2[0], 1usize).unwrap()).unwrap();
                    reg_119[0] = f64_to_word(if word_to_f64(reg_117[0]) < word_to_f64(reg_118[0]) { 1.0 } else { 0.0 });
                    pred_bb = 0;
                    bb = if truthy(reg_119[0]) { 1usize } else { 2usize };
                    continue;
                },
                1 => {
                    reg_121[0] = f64_to_word(1.0f64);
                    pred_bb = 1;
                    bb = 3usize;
                    continue;
                },
                2 => {
                    reg_122[0] = f64_to_word(0.0f64);
                    reg_123[0] = f64_to_word(1.0f64);
                    reg_124[0] = f64_to_word(word_to_f64(reg_122[0]) - word_to_f64(reg_123[0]));
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
                    return reg_125[0];
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
        let mut reg_110 = [0u64; 1];
        let mut reg_111 = [0u64; 1];
        reg_110[0] = f64_to_word(0.0f64);
        return reg_110[0];
    }

    fn dispatch___default_17_duty(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_17_duty();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_17_duty(&mut self) -> Word {
        let mut reg_112 = [0u64; 1];
        let mut reg_113 = [0u64; 1];
        reg_112[0] = f64_to_word(0.5f64);
        return reg_112[0];
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
        let arg_1 = [arg_1_value];
        let arg_2 = [arg_2_value];
        let mut reg_131 = [0u64; 1];
        let mut reg_132 = [0u64; 1];
        let mut reg_133 = [0u64; 1];
        let mut reg_134 = [0u64; 1];
        let mut reg_135 = [0u64; 1];
        let mut reg_136 = [0u64; 1];
        let mut reg_137 = [0u64; 1];
        let mut reg_138 = [0u64; 1];
        let mut reg_139 = [0u64; 1];
        let mut reg_140 = [0u64; 1];
        let mut reg_141 = [0u64; 1];
        let mut bb: usize = 0;
        let mut pred_bb: usize = 0;
        loop {
            match bb {
                0 => {
                    reg_131 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
                    reg_132 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
                    reg_133[0] = 7u64;
                    let call_result = self.osc_phasor(reg_131[0], reg_132[0]);
                    reg_134 = [call_result];
                    reg_135 = vec_to_words::<1>(self.memory.load(arg_2[0], 1usize).unwrap()).unwrap();
                    reg_136[0] = f64_to_word(if word_to_f64(reg_134[0]) < word_to_f64(reg_135[0]) { 1.0 } else { 0.0 });
                    pred_bb = 0;
                    bb = if truthy(reg_136[0]) { 1usize } else { 2usize };
                    continue;
                },
                1 => {
                    reg_138[0] = f64_to_word(1.0f64);
                    pred_bb = 1;
                    bb = 3usize;
                    continue;
                },
                2 => {
                    reg_139[0] = f64_to_word(0.0f64);
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
                    return reg_140[0];
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
        let mut reg_127 = [0u64; 1];
        let mut reg_128 = [0u64; 1];
        reg_127[0] = f64_to_word(0.0f64);
        return reg_127[0];
    }

    fn dispatch___default_20_duty(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_20_duty();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_20_duty(&mut self) -> Word {
        let mut reg_129 = [0u64; 1];
        let mut reg_130 = [0u64; 1];
        reg_129[0] = f64_to_word(0.5f64);
        return reg_129[0];
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
        let arg_1 = [arg_1_value];
        let mut reg_144 = [0u64; 1];
        let mut reg_145 = [0u64; 1];
        let mut reg_146 = [0u64; 1];
        let mut reg_147 = [0u64; 1];
        let mut reg_148 = [0u64; 1];
        let mut reg_149 = [0u64; 1];
        let mut reg_150 = [0u64; 1];
        let mut reg_151 = [0u64; 1];
        let mut reg_152 = [0u64; 1];
        let mut reg_153 = [0u64; 1];
        let mut reg_154 = [0u64; 1];
        reg_144 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_145 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
        reg_146[0] = 7u64;
        let call_result = self.osc_phasor(reg_144[0], reg_145[0]);
        reg_147 = [call_result];
        reg_148[0] = f64_to_word(2.0f64);
        reg_149[0] = f64_to_word(word_to_f64(reg_147[0]) * word_to_f64(reg_148[0]));
        reg_150[0] = 1u64;
        let call_result = self.math_PI();
        reg_151 = [call_result];
        reg_152[0] = f64_to_word(word_to_f64(reg_149[0]) * word_to_f64(reg_151[0]));
        reg_153[0] = f64_to_word(word_to_f64(reg_152[0]).sin());
        return reg_153[0];
    }

    fn dispatch___default_23_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_23_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_23_phase(&mut self) -> Word {
        let mut reg_142 = [0u64; 1];
        let mut reg_143 = [0u64; 1];
        reg_142[0] = f64_to_word(0.0f64);
        return reg_142[0];
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
        let arg_1 = [arg_1_value];
        let mut reg_157 = [0u64; 1];
        let mut reg_158 = [0u64; 1];
        let mut reg_159 = [0u64; 1];
        let mut reg_160 = [0u64; 1];
        let mut reg_161 = [0u64; 1];
        let mut reg_162 = [0u64; 1];
        let mut reg_163 = [0u64; 1];
        let mut reg_164 = [0u64; 1];
        let mut reg_165 = [0u64; 1];
        reg_157 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_158 = vec_to_words::<1>(self.memory.load(arg_1[0], 1usize).unwrap()).unwrap();
        reg_159[0] = 23u64;
        let call_result = self.osc_sinwave(reg_157[0], reg_158[0]);
        reg_160 = [call_result];
        reg_161[0] = f64_to_word(0.5f64);
        reg_162[0] = f64_to_word(word_to_f64(reg_160[0]) * word_to_f64(reg_161[0]));
        reg_163[0] = f64_to_word(0.5f64);
        reg_164[0] = f64_to_word(word_to_f64(reg_162[0]) + word_to_f64(reg_163[0]));
        return reg_164[0];
    }

    fn dispatch___default_25_phase(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.__default_25_phase();
        [result].to_vec()
    }

    #[inline(always)]
    fn __default_25_phase(&mut self) -> Word {
        let mut reg_155 = [0u64; 1];
        let mut reg_156 = [0u64; 1];
        reg_155[0] = f64_to_word(0.0f64);
        return reg_155[0];
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
        let mut reg_166 = [0u64; 1];
        let mut reg_167 = [0u64; 1];
        let mut reg_168 = [0u64; 1];
        let mut reg_169 = [0u64; 1];
        let mut reg_170 = [0u64; 1];
        reg_166 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_167[0] = f64_to_word(0.0f64);
        reg_168[0] = 23u64;
        let call_result = self.osc_sinwave(reg_166[0], reg_167[0]);
        reg_169 = [call_result];
        return reg_169[0];
    }

    fn dispatch_dsp(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let result = self.dsp();
        let abi_words = [result.0, result.1];
        let result_handle = self.memory.alloc(2usize);
        self.memory.store(result_handle, &abi_words, 2usize).unwrap();
        vec![result_handle]
    }

    #[inline(always)]
    fn dsp(&mut self) -> (Word, Word) {
        let mut reg_171 = [0u64; 1];
        let mut reg_172 = [0u64; 1];
        let mut reg_173 = [0u64; 1];
        let mut reg_308 = [0u64; 1];
        let mut reg_309 = [0u64; 1];
        let mut reg_310 = [0u64; 1];
        let mut reg_311 = [0u64; 1];
        let mut reg_312 = [0u64; 1];
        let mut reg_313 = [0u64; 1];
        let mut reg_314 = [0u64; 1];
        let mut reg_315 = [0u64; 1];
        let mut reg_316 = [0u64; 1];
        let mut reg_317 = [0u64; 1];
        let mut reg_318 = [0u64; 1];
        let mut reg_319 = [0u64; 1];
        let mut reg_320 = [0u64; 2];
        reg_171[0] = f64_to_word(50.0f64);
        reg_172[0] = self.memory.alloc(1usize);
        self.memory.store(reg_172[0], &[reg_171[0]], 1usize).unwrap();
        reg_308 = vec_to_words::<1>(self.memory.load(reg_172[0], 1usize).unwrap()).unwrap();
        reg_309[0] = 29u64;
        let call_result = self.r(reg_308[0]);
        reg_310 = [call_result];
        reg_311[0] = self.memory.alloc(1usize);
        self.memory.store(reg_311[0], &[reg_310[0]], 1usize).unwrap();
        reg_313[0] = self.memory.alloc(2usize);
        reg_314 = vec_to_words::<1>(self.memory.load(reg_311[0], 1usize).unwrap()).unwrap();
        reg_315[0] = self.memory.get_element(reg_313[0], 0usize).unwrap();
        self.memory.store(reg_315[0], &[reg_314[0]], 1usize).unwrap();
        reg_317 = vec_to_words::<1>(self.memory.load(reg_311[0], 1usize).unwrap()).unwrap();
        reg_318[0] = self.memory.get_element(reg_313[0], 1usize).unwrap();
        self.memory.store(reg_318[0], &[reg_317[0]], 1usize).unwrap();
        return ({ let words = self.memory.load(reg_313[0], 2usize).unwrap(); (words[0], words[1]) });
    }

    fn dispatch_r(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.r(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn r(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_174 = [0u64; 1];
        let mut reg_175 = [0u64; 1];
        let mut reg_176 = [0u64; 1];
        let mut reg_177 = [0u64; 1];
        let mut reg_178 = [0u64; 1];
        let mut reg_179 = [0u64; 1];
        let mut reg_180 = [0u64; 1];
        let mut reg_302 = [0u64; 1];
        let mut reg_303 = [0u64; 1];
        let mut reg_304 = [0u64; 1];
        let mut reg_305 = [0u64; 1];
        let mut reg_306 = [0u64; 1];
        let mut reg_307 = [0u64; 1];
        reg_174 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_175[0] = f64_to_word(10.0f64);
        reg_176[0] = f64_to_word(word_to_f64(reg_174[0]) * word_to_f64(reg_175[0]));
        reg_177[0] = 27u64;
        let call_result = self.osc(reg_176[0]);
        reg_178 = [call_result];
        reg_179[0] = f64_to_word(10.0f64);
        reg_180[0] = f64_to_word(word_to_f64(reg_178[0]) / word_to_f64(reg_179[0]));
        reg_302[0] = 30u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_303[0] = self.closures.alloc(reg_302[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_304[0] = reg_303[0];
        reg_305[0] = reg_303[0];
        reg_306[0] = f64_to_word(word_to_f64(reg_180[0]) + word_to_f64(reg_303[0]));
        return reg_306[0];
    }

    fn dispatch_lambda_0(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_0(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_0(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_181 = [0u64; 1];
        let mut reg_182 = [0u64; 1];
        let mut reg_183 = [0u64; 1];
        let mut reg_184 = [0u64; 1];
        let mut reg_185 = [0u64; 1];
        let mut reg_186 = [0u64; 1];
        let mut reg_187 = [0u64; 1];
        let mut reg_296 = [0u64; 1];
        let mut reg_297 = [0u64; 1];
        let mut reg_298 = [0u64; 1];
        let mut reg_299 = [0u64; 1];
        let mut reg_300 = [0u64; 1];
        let mut reg_301 = [0u64; 1];
        reg_181 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_182[0] = f64_to_word(9.0f64);
        reg_183[0] = f64_to_word(word_to_f64(reg_181[0]) * word_to_f64(reg_182[0]));
        reg_184[0] = 27u64;
        let call_result = self.osc(reg_183[0]);
        reg_185 = [call_result];
        reg_186[0] = f64_to_word(9.0f64);
        reg_187[0] = f64_to_word(word_to_f64(reg_185[0]) / word_to_f64(reg_186[0]));
        reg_296[0] = 31u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_297[0] = self.closures.alloc(reg_296[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_298[0] = reg_297[0];
        reg_299[0] = reg_297[0];
        reg_300[0] = f64_to_word(word_to_f64(reg_187[0]) + word_to_f64(reg_297[0]));
        return reg_300[0];
    }

    fn dispatch_lambda_1(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_1(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_1(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_188 = [0u64; 1];
        let mut reg_189 = [0u64; 1];
        let mut reg_190 = [0u64; 1];
        let mut reg_191 = [0u64; 1];
        let mut reg_192 = [0u64; 1];
        let mut reg_193 = [0u64; 1];
        let mut reg_194 = [0u64; 1];
        let mut reg_290 = [0u64; 1];
        let mut reg_291 = [0u64; 1];
        let mut reg_292 = [0u64; 1];
        let mut reg_293 = [0u64; 1];
        let mut reg_294 = [0u64; 1];
        let mut reg_295 = [0u64; 1];
        reg_188 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_189[0] = f64_to_word(8.0f64);
        reg_190[0] = f64_to_word(word_to_f64(reg_188[0]) * word_to_f64(reg_189[0]));
        reg_191[0] = 27u64;
        let call_result = self.osc(reg_190[0]);
        reg_192 = [call_result];
        reg_193[0] = f64_to_word(8.0f64);
        reg_194[0] = f64_to_word(word_to_f64(reg_192[0]) / word_to_f64(reg_193[0]));
        reg_290[0] = 32u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_291[0] = self.closures.alloc(reg_290[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_292[0] = reg_291[0];
        reg_293[0] = reg_291[0];
        reg_294[0] = f64_to_word(word_to_f64(reg_194[0]) + word_to_f64(reg_291[0]));
        return reg_294[0];
    }

    fn dispatch_lambda_2(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_2(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_2(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_195 = [0u64; 1];
        let mut reg_196 = [0u64; 1];
        let mut reg_197 = [0u64; 1];
        let mut reg_198 = [0u64; 1];
        let mut reg_199 = [0u64; 1];
        let mut reg_200 = [0u64; 1];
        let mut reg_201 = [0u64; 1];
        let mut reg_284 = [0u64; 1];
        let mut reg_285 = [0u64; 1];
        let mut reg_286 = [0u64; 1];
        let mut reg_287 = [0u64; 1];
        let mut reg_288 = [0u64; 1];
        let mut reg_289 = [0u64; 1];
        reg_195 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_196[0] = f64_to_word(7.0f64);
        reg_197[0] = f64_to_word(word_to_f64(reg_195[0]) * word_to_f64(reg_196[0]));
        reg_198[0] = 27u64;
        let call_result = self.osc(reg_197[0]);
        reg_199 = [call_result];
        reg_200[0] = f64_to_word(7.0f64);
        reg_201[0] = f64_to_word(word_to_f64(reg_199[0]) / word_to_f64(reg_200[0]));
        reg_284[0] = 33u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_285[0] = self.closures.alloc(reg_284[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_286[0] = reg_285[0];
        reg_287[0] = reg_285[0];
        reg_288[0] = f64_to_word(word_to_f64(reg_201[0]) + word_to_f64(reg_285[0]));
        return reg_288[0];
    }

    fn dispatch_lambda_3(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_3(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_3(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_202 = [0u64; 1];
        let mut reg_203 = [0u64; 1];
        let mut reg_204 = [0u64; 1];
        let mut reg_205 = [0u64; 1];
        let mut reg_206 = [0u64; 1];
        let mut reg_207 = [0u64; 1];
        let mut reg_208 = [0u64; 1];
        let mut reg_278 = [0u64; 1];
        let mut reg_279 = [0u64; 1];
        let mut reg_280 = [0u64; 1];
        let mut reg_281 = [0u64; 1];
        let mut reg_282 = [0u64; 1];
        let mut reg_283 = [0u64; 1];
        reg_202 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_203[0] = f64_to_word(6.0f64);
        reg_204[0] = f64_to_word(word_to_f64(reg_202[0]) * word_to_f64(reg_203[0]));
        reg_205[0] = 27u64;
        let call_result = self.osc(reg_204[0]);
        reg_206 = [call_result];
        reg_207[0] = f64_to_word(6.0f64);
        reg_208[0] = f64_to_word(word_to_f64(reg_206[0]) / word_to_f64(reg_207[0]));
        reg_278[0] = 34u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_279[0] = self.closures.alloc(reg_278[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_280[0] = reg_279[0];
        reg_281[0] = reg_279[0];
        reg_282[0] = f64_to_word(word_to_f64(reg_208[0]) + word_to_f64(reg_279[0]));
        return reg_282[0];
    }

    fn dispatch_lambda_4(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_4(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_4(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_209 = [0u64; 1];
        let mut reg_210 = [0u64; 1];
        let mut reg_211 = [0u64; 1];
        let mut reg_212 = [0u64; 1];
        let mut reg_213 = [0u64; 1];
        let mut reg_214 = [0u64; 1];
        let mut reg_215 = [0u64; 1];
        let mut reg_272 = [0u64; 1];
        let mut reg_273 = [0u64; 1];
        let mut reg_274 = [0u64; 1];
        let mut reg_275 = [0u64; 1];
        let mut reg_276 = [0u64; 1];
        let mut reg_277 = [0u64; 1];
        reg_209 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_210[0] = f64_to_word(5.0f64);
        reg_211[0] = f64_to_word(word_to_f64(reg_209[0]) * word_to_f64(reg_210[0]));
        reg_212[0] = 27u64;
        let call_result = self.osc(reg_211[0]);
        reg_213 = [call_result];
        reg_214[0] = f64_to_word(5.0f64);
        reg_215[0] = f64_to_word(word_to_f64(reg_213[0]) / word_to_f64(reg_214[0]));
        reg_272[0] = 35u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_273[0] = self.closures.alloc(reg_272[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_274[0] = reg_273[0];
        reg_275[0] = reg_273[0];
        reg_276[0] = f64_to_word(word_to_f64(reg_215[0]) + word_to_f64(reg_273[0]));
        return reg_276[0];
    }

    fn dispatch_lambda_5(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_5(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_5(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_216 = [0u64; 1];
        let mut reg_217 = [0u64; 1];
        let mut reg_218 = [0u64; 1];
        let mut reg_219 = [0u64; 1];
        let mut reg_220 = [0u64; 1];
        let mut reg_221 = [0u64; 1];
        let mut reg_222 = [0u64; 1];
        let mut reg_266 = [0u64; 1];
        let mut reg_267 = [0u64; 1];
        let mut reg_268 = [0u64; 1];
        let mut reg_269 = [0u64; 1];
        let mut reg_270 = [0u64; 1];
        let mut reg_271 = [0u64; 1];
        reg_216 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_217[0] = f64_to_word(4.0f64);
        reg_218[0] = f64_to_word(word_to_f64(reg_216[0]) * word_to_f64(reg_217[0]));
        reg_219[0] = 27u64;
        let call_result = self.osc(reg_218[0]);
        reg_220 = [call_result];
        reg_221[0] = f64_to_word(4.0f64);
        reg_222[0] = f64_to_word(word_to_f64(reg_220[0]) / word_to_f64(reg_221[0]));
        reg_266[0] = 36u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_267[0] = self.closures.alloc(reg_266[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_268[0] = reg_267[0];
        reg_269[0] = reg_267[0];
        reg_270[0] = f64_to_word(word_to_f64(reg_222[0]) + word_to_f64(reg_267[0]));
        return reg_270[0];
    }

    fn dispatch_lambda_6(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_6(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_6(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_223 = [0u64; 1];
        let mut reg_224 = [0u64; 1];
        let mut reg_225 = [0u64; 1];
        let mut reg_226 = [0u64; 1];
        let mut reg_227 = [0u64; 1];
        let mut reg_228 = [0u64; 1];
        let mut reg_229 = [0u64; 1];
        let mut reg_260 = [0u64; 1];
        let mut reg_261 = [0u64; 1];
        let mut reg_262 = [0u64; 1];
        let mut reg_263 = [0u64; 1];
        let mut reg_264 = [0u64; 1];
        let mut reg_265 = [0u64; 1];
        reg_223 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_224[0] = f64_to_word(3.0f64);
        reg_225[0] = f64_to_word(word_to_f64(reg_223[0]) * word_to_f64(reg_224[0]));
        reg_226[0] = 27u64;
        let call_result = self.osc(reg_225[0]);
        reg_227 = [call_result];
        reg_228[0] = f64_to_word(3.0f64);
        reg_229[0] = f64_to_word(word_to_f64(reg_227[0]) / word_to_f64(reg_228[0]));
        reg_260[0] = 37u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_261[0] = self.closures.alloc(reg_260[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_262[0] = reg_261[0];
        reg_263[0] = reg_261[0];
        reg_264[0] = f64_to_word(word_to_f64(reg_229[0]) + word_to_f64(reg_261[0]));
        return reg_264[0];
    }

    fn dispatch_lambda_7(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_7(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_7(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_230 = [0u64; 1];
        let mut reg_231 = [0u64; 1];
        let mut reg_232 = [0u64; 1];
        let mut reg_233 = [0u64; 1];
        let mut reg_234 = [0u64; 1];
        let mut reg_235 = [0u64; 1];
        let mut reg_236 = [0u64; 1];
        let mut reg_254 = [0u64; 1];
        let mut reg_255 = [0u64; 1];
        let mut reg_256 = [0u64; 1];
        let mut reg_257 = [0u64; 1];
        let mut reg_258 = [0u64; 1];
        let mut reg_259 = [0u64; 1];
        reg_230 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_231[0] = f64_to_word(2.0f64);
        reg_232[0] = f64_to_word(word_to_f64(reg_230[0]) * word_to_f64(reg_231[0]));
        reg_233[0] = 27u64;
        let call_result = self.osc(reg_232[0]);
        reg_234 = [call_result];
        reg_235[0] = f64_to_word(2.0f64);
        reg_236[0] = f64_to_word(word_to_f64(reg_234[0]) / word_to_f64(reg_235[0]));
        reg_254[0] = 38u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_255[0] = self.closures.alloc(reg_254[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_256[0] = reg_255[0];
        reg_257[0] = reg_255[0];
        reg_258[0] = f64_to_word(word_to_f64(reg_236[0]) + word_to_f64(reg_255[0]));
        return reg_258[0];
    }

    fn dispatch_lambda_8(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_8(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_8(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_237 = [0u64; 1];
        let mut reg_238 = [0u64; 1];
        let mut reg_239 = [0u64; 1];
        let mut reg_240 = [0u64; 1];
        let mut reg_241 = [0u64; 1];
        let mut reg_242 = [0u64; 1];
        let mut reg_243 = [0u64; 1];
        let mut reg_248 = [0u64; 1];
        let mut reg_249 = [0u64; 1];
        let mut reg_250 = [0u64; 1];
        let mut reg_251 = [0u64; 1];
        let mut reg_252 = [0u64; 1];
        let mut reg_253 = [0u64; 1];
        reg_237 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_238[0] = f64_to_word(1.0f64);
        reg_239[0] = f64_to_word(word_to_f64(reg_237[0]) * word_to_f64(reg_238[0]));
        reg_240[0] = 27u64;
        let call_result = self.osc(reg_239[0]);
        reg_241 = [call_result];
        reg_242[0] = f64_to_word(1.0f64);
        reg_243[0] = f64_to_word(word_to_f64(reg_241[0]) / word_to_f64(reg_242[0]));
        reg_248[0] = 39u64;
        let mut closure_upvalues = Vec::new();
        let mut closure_indirect = Vec::new();
        reg_249[0] = self.closures.alloc(reg_248[0], closure_upvalues, closure_indirect, 1usize).unwrap();
        reg_250[0] = reg_249[0];
        reg_251[0] = reg_249[0];
        reg_252[0] = f64_to_word(word_to_f64(reg_243[0]) + word_to_f64(reg_249[0]));
        return reg_252[0];
    }

    fn dispatch_lambda_9(&mut self, args: &[Word]) -> Vec<Word> {
        let mut arg_offset = 0usize;
        let arg_0_words = copy_words::<1>(&args[arg_offset..arg_offset + 1]).unwrap();
        arg_offset += 1;
        let arg_0_value = arg_0_words[0];
        let result = self.lambda_9(arg_0_value);
        [result].to_vec()
    }

    #[inline(always)]
    fn lambda_9(&mut self, arg_0_value: Word) -> Word {
        let arg_0 = [arg_0_value];
        let mut reg_244 = [0u64; 1];
        let mut reg_245 = [0u64; 1];
        let mut reg_246 = [0u64; 1];
        let mut reg_247 = [0u64; 1];
        reg_244 = vec_to_words::<1>(self.memory.load(arg_0[0], 1usize).unwrap()).unwrap();
        reg_245[0] = 27u64;
        let call_result = self.osc(reg_244[0]);
        reg_246 = [call_result];
        return reg_246[0];
    }

}
