// The type has no layout on the line of `:`: it goes on the next line
const BASE64_URL_SAFE_FORGIVING_ENGINE_LONGER: base64::engine::general_purpose::GeneralPurposeEngine = base64::engine::general_purpose::GeneralPurpose::new(&base64::alphabet::URL_SAFE, CONFIG);
static BASE64_URL_SAFE_FORGIVING_ENGINE_LONG: base64::engine::general_purpose::GeneralPurposeEngine = base64::engine::general_purpose::GeneralPurpose::new(&base64::alphabet::URL_SAFE, CONFIG);

// A broken type on the line of `:` is preferred
static NAME_LONGER: aaaa::bbbbbbbbbb::cccccccccccccc::Ddddddddddddddddddddddddddddddddddddddddd<u8> = Ddddddddddddddd::new(1);

// Fits with the value on the next line
const NAME: aaaa::bbbbbbbbbb::cccccccccccccc::Ddddddddddddddddddddddddddddddddddddddddddddd = 1;

fn f() {
    let name: aaaa::bbbbbbbbbb::cccccccccccccc::Ddddddddddddddddddddddddddddddddddddddddddd<Eeeeeeeeeeeeeee, Fffffffffffffffffff> = Ddddddd::new(1);
    let name: HashMap<Eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee, Fffffffffffffffffffffffffffffffffffffffff> = HashMap::new();
    let Some(xxxxxx): Option<aaaa::bbbbbbbbbb::cccccccccccccc::Dddddddddddddddddddddddddddddddddddd> = y else {
        return;
    };
}
