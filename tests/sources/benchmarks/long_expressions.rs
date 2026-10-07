// Adversarial benchmark: long operator chains with mixed precedence, deeply
// nested `if`/`else` and blocks, long conditions, and items with long
// generics, where clauses and parameter lists. Operator chains can break
// before any operator and nested blocks multiply the layouts of their bodies.

fn operator_chains() {
    let sum = aaaaaaaa + bbbbbbbb + cccccccc + dddddddd + eeeeeeee + ffffffff + gggggggg + hhhhhhhh + iiiiiiii + jjjjjjjj + kkkkkkkk + llllllll + mmmmmmmm + nnnnnnnn + oooooooo + pppppppp;
    let mixed = aaaaaaaa * bbbbbbbb + cccccccc * dddddddd - eeeeeeee / ffffffff + gggggggg % hhhhhhhh * iiiiiiii - jjjjjjjj * kkkkkkkk + llllllll / mmmmmmmm - nnnnnnnn * oooooooo;
    let cond = first_condition && second_condition || third_condition && fourth_condition || fifth_condition && sixth_condition && seventh_condition || eighth_condition;
    let cmp = (aaaaaaaa + bbbbbbbb) * (cccccccc - dddddddd) >= (eeeeeeee + ffffffff) * (gggggggg - hhhhhhhh) && (iiiiiiii | jjjjjjjj) & (kkkkkkkk ^ llllllll) != (mmmmmmmm << 4) >> 2;
    let calls = compute(aaaaaaaa, bbbbbbbb) + compute(cccccccc, dddddddd) * compute(eeeeeeee, ffffffff) - compute(gggggggg, hhhhhhhh) / compute(iiiiiiii, jjjjjjjj) + compute(kkkkkkkk, llllllll);
    let casts = aaaaaaaa as u64 + bbbbbbbb as u64 * cccccccc as u64 - dddddddd as u64 + eeeeeeee as u64 * ffffffff as u64 - gggggggg as u64 + hhhhhhhh as u64 * iiiiiiii as u64;
    let refs = &mut *values[aaaaaaaa + bbbbbbbb * cccccccc - dddddddd][eeeeeeee * ffffffff + gggggggg][hhhhhhhh - iiiiiiii * jjjjjjjj + kkkkkkkk];
    let ranges = &slice[start_index + offset * stride..end_index - offset * stride + remainder_of_division];
    total += first_component * weight_one + second_component * weight_two + third_component * weight_three + fourth_component * weight_four;
    let nested = (((((((((aaaaaaaa + bbbbbbbb) * cccccccc) - dddddddd) / eeeeeeee) + ffffffff) * gggggggg) - hhhhhhhh) / iiiiiiii) + jjjjjjjj) * kkkkkkkk + llllllll * mmmmmmmm;
}

fn nested_conditions(a: u32, b: u32, c: u32) -> u32 {
    if a > b && b > c && c > minimum_value && a < maximum_value && b < maximum_value && c != forbidden_value {
        if a % 2 == 0 {
            if b % 3 == 0 {
                if c % 5 == 0 { compute_something(a, b, c) } else if c % 7 == 0 { compute_other(a, b, c) } else { 0 }
            } else if b % 11 == 0 {
                if let Some(value) = lookup(a, b, c) { value + adjustment(a) * factor(b) - correction(c) } else { fallback(a, b, c) }
            } else {
                { { { { { { a + b + c + first_extra_value + second_extra_value + third_extra_value } } } } } }
            }
        } else {
            let x = if a > b { if b > c { a } else { c } } else { if a > c { b } else { if b > c { c } else { a + b + c } } };
            let y = if first_condition_is_true(a) && second_condition_is_true(b) { compute_first(a, b) } else { compute_second(b, c) };
            x + y
        }
    } else if a == b {
        return if c > 0 { a * c + b * c + a * b + first_long_value + second_long_value } else { a + b };
    } else {
        loop_like(|step| if step > limit { break_value(step, limit) } else { continue_value(step, limit, increment) })
    }
}

struct Container<'a, Key: Hash + Eq + Clone + Debug, Value: Clone + Debug + Default, Hasher: BuildHasher + Default> {
    entries: HashMap<Key, Vec<(Value, Option<Box<dyn Fn(&Key, &Value) -> Result<Value, Error> + Send + Sync + 'a>>)>, Hasher>,
    cache: Option<BTreeMap<Key, Arc<Mutex<Vec<Option<Result<Value, Box<dyn std::error::Error + Send + Sync>>>>>>>>,
}

enum Message<Payload: Clone + Debug + Send + Sync + 'static, Metadata: Clone + Debug + Default + PartialEq> {
    Request { identifier: u64, payload: Payload, metadata: Metadata, deadline: Option<Duration>, retries: u32 },
    Response(u64, Result<Payload, (ErrorCode, String, Option<Metadata>, Vec<(String, String)>)>),
    Broadcast(Vec<(Recipient, Payload)>, Metadata, Priority, Option<Box<dyn Fn(&Payload) -> bool>>),
}

impl<'a, Key: Hash + Eq + Clone + Debug, Value: Clone + Debug + Default, Hasher: BuildHasher + Default> Container<'a, Key, Value, Hasher> where Key: Ord + Send + Sync, Value: Send + Sync + PartialEq {
    pub fn insert_with_callback(&mut self, key: Key, value: Value, callback: Option<Box<dyn Fn(&Key, &Value) -> Result<Value, Error> + Send + Sync + 'a>>, overwrite: bool) -> Result<Option<Value>, Error> {
        let previous = self.entries.get(&key).and_then(|values| values.iter().find(|(existing, _)| *existing == value).map(|(existing, _)| existing.clone()));
        if overwrite || previous.is_none() && self.cache.as_ref().map_or(true, |cache| !cache.contains_key(&key)) {
            self.entries.entry(key.clone()).or_insert_with(Vec::new).push((value.clone(), callback));
        }
        Ok(previous)
    }

    fn generic_helper<T: Clone + Debug, U: From<T> + Into<Value>, F: Fn(T) -> U, G: FnMut(U) -> Option<T>>(first: T, second: F, third: G) -> Option<(T, U)> where T: Send, U: Sync {
        let converted = second(first.clone());
        third(converted).map(|back| (back, second(first)))
    }
}
