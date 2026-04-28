// Mixed-language sample used by tests/integration.rs.
// English identifiers should be ignored; Pinyin ones should be detected.

fn huo_qu_yong_hu() -> i32 {
    42
}

fn load_file(path: &str) -> &str {
    path
}

struct GuanLiYuan {
    zi_duan: i32,
}

enum ZhuangTai {
    JinXing,
    WanCheng,
}

const SHU_LIANG: i32 = 10;
