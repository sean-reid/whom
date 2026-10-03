pub const MULTIPLE_LANGUAGES: &str = "Q20923490";

pub fn family(lang_qid: &str) -> Option<&'static str> {
    TABLE
        .binary_search_by_key(&qid_number(lang_qid)?, |(q, _)| qid_number(q).unwrap_or(0))
        .ok()
        .map(|i| TABLE[i].1)
}

fn qid_number(q: &str) -> Option<u64> {
    q.strip_prefix('Q')?.parse().ok()
}

// Sorted by numeric qid; `family` binary-searches it.
const TABLE: &[(&str, &str)] = &[
    ("Q143", "constructed"),
    ("Q150", "romance"),
    ("Q188", "germanic"),
    ("Q256", "turkic"),
    ("Q294", "germanic"),
    ("Q397", "romance"),
    ("Q652", "romance"),
    ("Q809", "slavic"),
    ("Q1321", "romance"),
    ("Q1412", "uralic"),
    ("Q1568", "indo-aryan"),
    ("Q1571", "indo-aryan"),
    ("Q1617", "indo-aryan"),
    ("Q1860", "germanic"),
    ("Q4627", "aymaran"),
    ("Q5137", "indo-aryan"),
    ("Q5146", "romance"),
    ("Q5218", "quechuan"),
    ("Q5287", "japonic"),
    ("Q5885", "dravidian"),
    ("Q6654", "slavic"),
    ("Q7026", "romance"),
    ("Q7411", "germanic"),
    ("Q7737", "slavic"),
    ("Q7838", "bantu"),
    ("Q7850", "sinitic"),
    ("Q7913", "romance"),
    ("Q7918", "slavic"),
    ("Q7930", "austronesian"),
    ("Q8097", "dravidian"),
    ("Q8108", "kartvelian"),
    ("Q8641", "germanic"),
    ("Q8748", "albanian"),
    ("Q8752", "basque"),
    ("Q8765", "romance"),
    ("Q8785", "armenian"),
    ("Q8798", "slavic"),
    ("Q8821", "slavic"),
    ("Q9027", "germanic"),
    ("Q9035", "germanic"),
    ("Q9043", "germanic"),
    ("Q9051", "germanic"),
    ("Q9056", "slavic"),
    ("Q9058", "slavic"),
    ("Q9063", "slavic"),
    ("Q9067", "uralic"),
    ("Q9072", "uralic"),
    ("Q9078", "baltic"),
    ("Q9083", "baltic"),
    ("Q9091", "slavic"),
    ("Q9129", "hellenic"),
    ("Q9142", "celtic"),
    ("Q9166", "semitic"),
    ("Q9168", "iranian"),
    ("Q9176", "koreanic"),
    ("Q9186", "sinitic"),
    ("Q9192", "sinitic"),
    ("Q9199", "vietic"),
    ("Q9205", "austroasiatic"),
    ("Q9211", "tai"),
    ("Q9217", "tai"),
    ("Q9228", "tibeto-burman"),
    ("Q9237", "austronesian"),
    ("Q9240", "austronesian"),
    ("Q9246", "mongolic"),
    ("Q9252", "turkic"),
    ("Q9255", "turkic"),
    ("Q9260", "iranian"),
    ("Q9264", "turkic"),
    ("Q9267", "turkic"),
    ("Q9288", "semitic"),
    ("Q9292", "turkic"),
    ("Q9296", "slavic"),
    ("Q9299", "slavic"),
    ("Q9301", "slavic"),
    ("Q9303", "slavic"),
    ("Q9307", "romance"),
    ("Q9309", "celtic"),
    ("Q9314", "celtic"),
    ("Q9610", "indo-aryan"),
    ("Q10179", "bantu"),
    ("Q11051", "indo-aryan"),
    ("Q11059", "indo-aryan"),
    ("Q12107", "celtic"),
    ("Q12175", "celtic"),
    ("Q13199", "romance"),
    ("Q13218", "bantu"),
    ("Q13248", "slavic"),
    ("Q13263", "turkic"),
    ("Q13267", "indo-aryan"),
    ("Q13275", "cushitic"),
    ("Q13286", "slavic"),
    ("Q13300", "uto-aztecan"),
    ("Q13310", "athabaskan"),
    ("Q13389", "turkic"),
    ("Q13955", "semitic"),
    ("Q14185", "romance"),
    ("Q14196", "germanic"),
    ("Q14549", "germanic"),
    ("Q25164", "germanic"),
    ("Q25167", "germanic"),
    ("Q25258", "germanic"),
    ("Q25285", "turkic"),
    ("Q25289", "celtic"),
    ("Q25355", "eskimo-aleut"),
    ("Q25433", "germanic"),
    ("Q26245", "slavic"),
    ("Q27175", "germanic"),
    ("Q28026", "kwa"),
    ("Q28244", "semitic"),
    ("Q28602", "semitic"),
    ("Q29401", "indo-aryan"),
    ("Q29507", "romance"),
    ("Q29919", "semitic"),
    ("Q29921", "eskimo-aleut"),
    ("Q33049", "iranian"),
    ("Q33111", "romance"),
    ("Q33239", "austronesian"),
    ("Q33268", "indo-aryan"),
    ("Q33273", "bantu"),
    ("Q33295", "austronesian"),
    ("Q33298", "austronesian"),
    ("Q33348", "turkic"),
    ("Q33375", "sinitic"),
    ("Q33388", "iroquoian"),
    ("Q33441", "romance"),
    ("Q33549", "austronesian"),
    ("Q33557", "uralic"),
    ("Q33569", "austronesian"),
    ("Q33573", "bantu"),
    ("Q33578", "volta-niger"),
    ("Q33587", "bantu"),
    ("Q33673", "dravidian"),
    ("Q33810", "indo-aryan"),
    ("Q33823", "indo-aryan"),
    ("Q33845", "romance"),
    ("Q33864", "cushitic"),
    ("Q33947", "uralic"),
    ("Q33968", "iranian"),
    ("Q33973", "romance"),
    ("Q33976", "romance"),
    ("Q33997", "indo-aryan"),
    ("Q34004", "bantu"),
    ("Q34011", "austronesian"),
    ("Q34057", "austronesian"),
    ("Q34094", "austronesian"),
    ("Q34124", "semitic"),
    ("Q34137", "bantu"),
    ("Q34239", "indo-aryan"),
    ("Q34251", "dravidian"),
    ("Q34257", "senegambian"),
    ("Q34271", "tibeto-burman"),
    ("Q34290", "sinitic"),
    ("Q34311", "volta-niger"),
    ("Q34340", "bantu"),
    ("Q35497", "hellenic"),
    ("Q35499", "slavic"),
    ("Q35876", "tupian"),
    ("Q36109", "indo-aryan"),
    ("Q36217", "bantu"),
    ("Q36236", "dravidian"),
    ("Q36368", "iranian"),
    ("Q36451", "austronesian"),
    ("Q36495", "sinitic"),
    ("Q37041", "sinitic"),
    ("Q56475", "chadic"),
    ("Q58635", "indo-aryan"),
    ("Q58680", "iranian"),
    ("Q178440", "iranian"),
    ("Q387066", "germanic"),
    ("Q727694", "sinitic"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_and_unique() {
        let nums: Vec<u64> = TABLE.iter().map(|(q, _)| qid_number(q).unwrap()).collect();
        let mut sorted = nums.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(nums, sorted);
        assert!(TABLE.len() >= 60);
    }

    #[test]
    fn top_ten_pool_languages_have_families() {
        let expected = [
            ("Q7411", "germanic"),
            ("Q1860", "germanic"),
            ("Q652", "romance"),
            ("Q1321", "romance"),
            ("Q150", "romance"),
            ("Q5287", "japonic"),
            ("Q188", "germanic"),
            ("Q809", "slavic"),
            ("Q9027", "germanic"),
            ("Q13955", "semitic"),
        ];
        for (q, f) in expected {
            assert_eq!(family(q), Some(f), "{q}");
        }
    }

    #[test]
    fn unknown_language_has_no_family() {
        assert_eq!(family("Q999999999"), None);
        assert_eq!(family(MULTIPLE_LANGUAGES), None);
        assert_eq!(family("not a qid"), None);
    }
}
