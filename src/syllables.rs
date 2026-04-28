//! Mandarin Pinyin syllable inventory (no tones).
//!
//! The list below is compiled from the standard Mandarin Pinyin syllable
//! table (see <https://en.wikipedia.org/wiki/Pinyin_table>). Every entry is
//! a fully-formed syllable: an optional initial concatenated with a final.
//!
//! `lv` / `lve` / `nv` / `nve` are included as common keyboard substitutes
//! for `lü` / `lüe` / `nü` / `nüe`, since identifiers in code rarely contain
//! the literal `ü` character.
//!
//! The set is exposed through [`syllable_set`] and [`max_syllable_len`].

use std::collections::HashSet;
use std::sync::OnceLock;

/// Static list of every recognized Pinyin syllable, lowercase, no tones.
const SYLLABLES: &[&str] = &[
    // No-initial / y- / w- onsets
    "a", "ai", "an", "ang", "ao", "e", "ei", "en", "eng", "er", "o", "ou",
    "yi", "ya", "yo", "ye", "yao", "you", "yan", "yin", "yang", "ying", "yong", "yai",
    "wu", "wa", "wo", "wai", "wei", "wan", "wen", "wang", "weng",
    "yu", "yue", "yuan", "yun",
    // b
    "ba", "bo", "bai", "bei", "bao", "ban", "ben", "bang", "beng",
    "bi", "bie", "biao", "bian", "bin", "bing", "bu", "biang",
    // p
    "pa", "po", "pai", "pei", "pao", "pou", "pan", "pen", "pang", "peng",
    "pi", "pie", "piao", "pian", "pin", "ping", "pu",
    // m
    "ma", "mo", "me", "mai", "mei", "mao", "mou", "man", "men", "mang", "meng",
    "mi", "mie", "miao", "miu", "mian", "min", "ming", "mu",
    // f
    "fa", "fo", "fei", "fou", "fan", "fen", "fang", "feng", "fu", "fiao",
    // d
    "da", "de", "dai", "dei", "dao", "dou", "dan", "den", "dang", "deng", "dong",
    "di", "dia", "die", "diao", "diu", "dian", "ding",
    "du", "duo", "dui", "duan", "dun",
    // t
    "ta", "te", "tai", "tei", "tao", "tou", "tan", "tang", "teng", "tong",
    "ti", "tie", "tiao", "tian", "ting",
    "tu", "tuo", "tui", "tuan", "tun",
    // n
    "na", "ne", "nai", "nei", "nao", "nou", "nan", "nen", "nang", "neng", "nong",
    "ni", "nia", "nie", "niao", "niu", "nian", "nin", "niang", "ning",
    "nu", "nuo", "nuan",
    "nv", "nve",
    // l
    "la", "le", "lo", "lai", "lei", "lao", "lou", "lan", "lang", "leng", "long",
    "li", "lia", "lie", "liao", "liu", "lian", "lin", "liang", "ling",
    "lu", "luo", "luan", "lun",
    "lv", "lve",
    // g
    "ga", "ge", "gai", "gei", "gao", "gou", "gan", "gen", "gang", "geng", "gong",
    "gu", "gua", "guai", "guan", "guang", "gui", "gun", "guo",
    // k
    "ka", "ke", "kai", "kao", "kou", "kan", "ken", "kang", "keng", "kong", "kei",
    "ku", "kua", "kuai", "kuan", "kuang", "kui", "kun", "kuo",
    // h
    "ha", "he", "hai", "hei", "hao", "hou", "han", "hen", "hang", "heng", "hong",
    "hu", "hua", "huai", "huan", "huang", "hui", "hun", "huo",
    // j
    "ji", "jia", "jie", "jiao", "jiu", "jian", "jin", "jiang", "jing", "jiong",
    "ju", "juan", "jun", "jue",
    // q
    "qi", "qia", "qie", "qiao", "qiu", "qian", "qin", "qiang", "qing", "qiong",
    "qu", "quan", "qun", "que",
    // x
    "xi", "xia", "xie", "xiao", "xiu", "xian", "xin", "xiang", "xing", "xiong",
    "xu", "xuan", "xun", "xue",
    // zh
    "zha", "zhe", "zhi", "zhai", "zhei", "zhao", "zhou", "zhan", "zhen",
    "zhang", "zheng", "zhong",
    "zhu", "zhua", "zhuai", "zhuan", "zhuang", "zhun", "zhui", "zhuo",
    // ch
    "cha", "che", "chi", "chai", "chao", "chou", "chan", "chen",
    "chang", "cheng", "chong",
    "chu", "chua", "chuai", "chuan", "chuang", "chun", "chui", "chuo",
    // sh
    "sha", "she", "shi", "shai", "shei", "shao", "shou", "shan", "shen",
    "shang", "sheng",
    "shu", "shua", "shuai", "shuan", "shuang", "shun", "shui", "shuo",
    // r
    "re", "ri", "rao", "rou", "ran", "ren", "rang", "reng", "rong",
    "ru", "rua", "rui", "ruan", "run", "ruo",
    // z
    "za", "ze", "zi", "zai", "zao", "zan", "zou", "zang", "zei", "zen", "zeng", "zong",
    "zu", "zuo", "zui", "zuan", "zun",
    // c
    "ca", "ce", "ci", "cai", "cao", "cou", "can", "cen", "cang", "ceng", "cong",
    "cu", "cuo", "cui", "cuan", "cun",
    // s
    "sa", "se", "si", "sai", "sao", "sou", "san", "sen", "sang", "seng", "song",
    "su", "suo", "sui", "suan", "sun",
];

/// Returns the global syllable set, lazily constructed once.
pub fn syllable_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| SYLLABLES.iter().copied().collect())
}

/// Returns the longest syllable length in bytes, lazily computed once.
pub fn max_syllable_len() -> usize {
    static MAX: OnceLock<usize> = OnceLock::new();
    *MAX.get_or_init(|| SYLLABLES.iter().map(|s| s.len()).max().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_contains_common_syllables() {
        let set = syllable_set();
        for s in [
            "huo", "qu", "xin", "xi", "yong", "hu", "zhang", "wang", "li", "chen",
            "shuang", "liang", "xian", "ying", "lv", "nv",
        ] {
            assert!(set.contains(s), "{s} should be in the syllable set");
        }
    }

    #[test]
    fn set_excludes_non_syllables() {
        let set = syllable_set();
        for s in ["xz", "qx", "vy", "kx", "rr", "bb"] {
            assert!(!set.contains(s), "{s} should NOT be in the syllable set");
        }
    }

    #[test]
    fn max_syllable_len_is_six() {
        // longest standard syllables (chuang/zhuang/shuang) are 6 bytes
        assert_eq!(max_syllable_len(), 6);
    }
}
