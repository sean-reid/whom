use crate::text::normalize;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Subregion {
    pub slug: &'static str,
    pub label: &'static str,
    pub continent: &'static str,
}

pub const SUBREGIONS: &[Subregion] = &[
    Subregion {
        slug: "western-europe",
        label: "Western Europe",
        continent: "europe",
    },
    Subregion {
        slug: "northern-europe",
        label: "Northern Europe",
        continent: "europe",
    },
    Subregion {
        slug: "southern-europe",
        label: "Southern Europe",
        continent: "europe",
    },
    Subregion {
        slug: "eastern-europe",
        label: "Eastern Europe",
        continent: "europe",
    },
    Subregion {
        slug: "northern-america",
        label: "Northern America",
        continent: "americas",
    },
    Subregion {
        slug: "central-america",
        label: "Central America",
        continent: "americas",
    },
    Subregion {
        slug: "caribbean",
        label: "Caribbean",
        continent: "americas",
    },
    Subregion {
        slug: "south-america",
        label: "South America",
        continent: "americas",
    },
    Subregion {
        slug: "eastern-asia",
        label: "Eastern Asia",
        continent: "asia",
    },
    Subregion {
        slug: "south-eastern-asia",
        label: "South-eastern Asia",
        continent: "asia",
    },
    Subregion {
        slug: "southern-asia",
        label: "Southern Asia",
        continent: "asia",
    },
    Subregion {
        slug: "central-asia",
        label: "Central Asia",
        continent: "asia",
    },
    Subregion {
        slug: "western-asia",
        label: "Western Asia",
        continent: "asia",
    },
    Subregion {
        slug: "northern-africa",
        label: "Northern Africa",
        continent: "africa",
    },
    Subregion {
        slug: "western-africa",
        label: "Western Africa",
        continent: "africa",
    },
    Subregion {
        slug: "eastern-africa",
        label: "Eastern Africa",
        continent: "africa",
    },
    Subregion {
        slug: "middle-africa",
        label: "Middle Africa",
        continent: "africa",
    },
    Subregion {
        slug: "southern-africa",
        label: "Southern Africa",
        continent: "africa",
    },
    Subregion {
        slug: "australia-and-new-zealand",
        label: "Australia and New Zealand",
        continent: "oceania",
    },
    Subregion {
        slug: "melanesia",
        label: "Melanesia",
        continent: "oceania",
    },
    Subregion {
        slug: "micronesia",
        label: "Micronesia",
        continent: "oceania",
    },
    Subregion {
        slug: "polynesia",
        label: "Polynesia",
        continent: "oceania",
    },
];

pub fn subregion(slug: &str) -> Option<&'static Subregion> {
    SUBREGIONS.iter().find(|s| s.slug == slug)
}

pub fn region_of_qid(qid: &str) -> Option<&'static str> {
    by_qid().get(qid).copied()
}

// Pool records carry citizenship as English labels, matched here after normalize.
pub fn region_of_label(label: &str) -> Option<&'static str> {
    by_label().get(normalize(label).as_str()).copied()
}

fn by_qid() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| COUNTRIES.iter().map(|(q, _, r)| (*q, *r)).collect())
}

fn by_label() -> &'static HashMap<String, &'static str> {
    static MAP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        COUNTRIES
            .iter()
            .map(|(_, l, r)| (normalize(l), *r))
            .collect()
    })
}

// (Wikidata qid, English label, UN geoscheme subregion). Historical states map
// to the subregion of their main successor.
const COUNTRIES: &[(&str, &str, &str)] = &[
    // Western Europe
    ("Q142", "France", "western-europe"),
    ("Q183", "Germany", "western-europe"),
    ("Q31", "Belgium", "western-europe"),
    ("Q55", "Netherlands", "western-europe"),
    ("Q29999", "Kingdom of the Netherlands", "western-europe"),
    ("Q40", "Austria", "western-europe"),
    ("Q39", "Switzerland", "western-europe"),
    ("Q32", "Luxembourg", "western-europe"),
    ("Q235", "Monaco", "western-europe"),
    ("Q347", "Liechtenstein", "western-europe"),
    ("Q16957", "German Democratic Republic", "western-europe"),
    ("Q713750", "West Germany", "western-europe"),
    ("Q7318", "Nazi Germany", "western-europe"),
    ("Q41304", "Weimar Republic", "western-europe"),
    ("Q43287", "German Empire", "western-europe"),
    ("Q153015", "Kingdom of Prussia", "western-europe"),
    ("Q28513", "Austria-Hungary", "western-europe"),
    ("Q131964", "Austrian Empire", "western-europe"),
    ("Q12548", "Holy Roman Empire", "western-europe"),
    ("Q70802", "French Third Republic", "western-europe"),
    ("Q3024240", "Vichy France", "western-europe"),
    // Northern Europe
    ("Q145", "United Kingdom", "northern-europe"),
    ("Q21", "England", "northern-europe"),
    ("Q22", "Scotland", "northern-europe"),
    ("Q25", "Wales", "northern-europe"),
    ("Q26", "Northern Ireland", "northern-europe"),
    (
        "Q174193",
        "United Kingdom of Great Britain and Ireland",
        "northern-europe",
    ),
    ("Q161885", "Kingdom of Great Britain", "northern-europe"),
    ("Q8680", "British Empire", "northern-europe"),
    ("Q27", "Ireland", "northern-europe"),
    ("Q20", "Norway", "northern-europe"),
    ("Q34", "Sweden", "northern-europe"),
    ("Q35", "Denmark", "northern-europe"),
    ("Q33", "Finland", "northern-europe"),
    ("Q189", "Iceland", "northern-europe"),
    ("Q37", "Lithuania", "northern-europe"),
    ("Q211", "Latvia", "northern-europe"),
    ("Q191", "Estonia", "northern-europe"),
    // Southern Europe
    ("Q38", "Italy", "southern-europe"),
    ("Q172579", "Kingdom of Italy", "southern-europe"),
    ("Q29", "Spain", "southern-europe"),
    ("Q45", "Portugal", "southern-europe"),
    ("Q41", "Greece", "southern-europe"),
    ("Q215", "Slovenia", "southern-europe"),
    ("Q224", "Croatia", "southern-europe"),
    ("Q225", "Bosnia and Herzegovina", "southern-europe"),
    ("Q403", "Serbia", "southern-europe"),
    ("Q236", "Montenegro", "southern-europe"),
    ("Q221", "North Macedonia", "southern-europe"),
    ("Q222", "Albania", "southern-europe"),
    ("Q1246", "Kosovo", "southern-europe"),
    ("Q233", "Malta", "southern-europe"),
    ("Q228", "Andorra", "southern-europe"),
    ("Q238", "San Marino", "southern-europe"),
    ("Q237", "Vatican City", "southern-europe"),
    ("Q36704", "Yugoslavia", "southern-europe"),
    (
        "Q83286",
        "Socialist Federal Republic of Yugoslavia",
        "southern-europe",
    ),
    ("Q170072", "Kingdom of Yugoslavia", "southern-europe"),
    ("Q37024", "Serbia and Montenegro", "southern-europe"),
    // Eastern Europe
    ("Q159", "Russia", "eastern-europe"),
    ("Q15180", "Soviet Union", "eastern-europe"),
    ("Q34266", "Russian Empire", "eastern-europe"),
    (
        "Q2184",
        "Russian Soviet Federative Socialist Republic",
        "eastern-europe",
    ),
    ("Q212", "Ukraine", "eastern-europe"),
    ("Q184", "Belarus", "eastern-europe"),
    ("Q217", "Moldova", "eastern-europe"),
    ("Q36", "Poland", "eastern-europe"),
    ("Q1206012", "Second Polish Republic", "eastern-europe"),
    ("Q213", "Czech Republic", "eastern-europe"),
    ("Q33946", "Czechoslovakia", "eastern-europe"),
    ("Q214", "Slovakia", "eastern-europe"),
    ("Q28", "Hungary", "eastern-europe"),
    ("Q218", "Romania", "eastern-europe"),
    ("Q219", "Bulgaria", "eastern-europe"),
    // Northern America
    ("Q30", "United States of America", "northern-america"),
    ("Q16", "Canada", "northern-america"),
    ("Q223", "Greenland", "northern-america"),
    ("Q126125", "Bermuda", "northern-america"),
    // Central America
    ("Q96", "Mexico", "central-america"),
    ("Q774", "Guatemala", "central-america"),
    ("Q783", "Honduras", "central-america"),
    ("Q792", "El Salvador", "central-america"),
    ("Q811", "Nicaragua", "central-america"),
    ("Q800", "Costa Rica", "central-america"),
    ("Q804", "Panama", "central-america"),
    ("Q242", "Belize", "central-america"),
    // Caribbean
    ("Q241", "Cuba", "caribbean"),
    ("Q786", "Dominican Republic", "caribbean"),
    ("Q790", "Haiti", "caribbean"),
    ("Q766", "Jamaica", "caribbean"),
    ("Q1183", "Puerto Rico", "caribbean"),
    ("Q754", "Trinidad and Tobago", "caribbean"),
    ("Q244", "Barbados", "caribbean"),
    ("Q778", "The Bahamas", "caribbean"),
    ("Q781", "Antigua and Barbuda", "caribbean"),
    ("Q784", "Dominica", "caribbean"),
    ("Q769", "Grenada", "caribbean"),
    ("Q763", "Saint Kitts and Nevis", "caribbean"),
    ("Q760", "Saint Lucia", "caribbean"),
    ("Q757", "Saint Vincent and the Grenadines", "caribbean"),
    // South America
    ("Q155", "Brazil", "south-america"),
    ("Q414", "Argentina", "south-america"),
    ("Q298", "Chile", "south-america"),
    ("Q77", "Uruguay", "south-america"),
    ("Q733", "Paraguay", "south-america"),
    ("Q750", "Bolivia", "south-america"),
    ("Q419", "Peru", "south-america"),
    ("Q736", "Ecuador", "south-america"),
    ("Q739", "Colombia", "south-america"),
    ("Q717", "Venezuela", "south-america"),
    ("Q734", "Guyana", "south-america"),
    ("Q730", "Suriname", "south-america"),
    // Eastern Asia
    ("Q148", "People's Republic of China", "eastern-asia"),
    ("Q8733", "Qing dynasty", "eastern-asia"),
    ("Q179876", "Republic of China (1912–1949)", "eastern-asia"),
    ("Q865", "Taiwan", "eastern-asia"),
    ("Q8646", "Hong Kong", "eastern-asia"),
    ("Q14773", "Macau", "eastern-asia"),
    ("Q17", "Japan", "eastern-asia"),
    ("Q188712", "Empire of Japan", "eastern-asia"),
    ("Q884", "South Korea", "eastern-asia"),
    ("Q423", "North Korea", "eastern-asia"),
    ("Q28233", "Korea under Japanese rule", "eastern-asia"),
    ("Q711", "Mongolia", "eastern-asia"),
    // South-eastern Asia
    ("Q869", "Thailand", "south-eastern-asia"),
    ("Q881", "Vietnam", "south-eastern-asia"),
    ("Q928", "Philippines", "south-eastern-asia"),
    ("Q252", "Indonesia", "south-eastern-asia"),
    ("Q207521", "Dutch East Indies", "south-eastern-asia"),
    ("Q833", "Malaysia", "south-eastern-asia"),
    ("Q334", "Singapore", "south-eastern-asia"),
    ("Q836", "Myanmar", "south-eastern-asia"),
    ("Q424", "Cambodia", "south-eastern-asia"),
    ("Q819", "Laos", "south-eastern-asia"),
    ("Q921", "Brunei", "south-eastern-asia"),
    ("Q574", "East Timor", "south-eastern-asia"),
    // Southern Asia
    ("Q668", "India", "southern-asia"),
    ("Q129286", "British Raj", "southern-asia"),
    ("Q843", "Pakistan", "southern-asia"),
    ("Q902", "Bangladesh", "southern-asia"),
    ("Q854", "Sri Lanka", "southern-asia"),
    ("Q837", "Nepal", "southern-asia"),
    ("Q917", "Bhutan", "southern-asia"),
    ("Q826", "Maldives", "southern-asia"),
    ("Q889", "Afghanistan", "southern-asia"),
    ("Q794", "Iran", "southern-asia"),
    // Central Asia
    ("Q232", "Kazakhstan", "central-asia"),
    ("Q265", "Uzbekistan", "central-asia"),
    ("Q813", "Kyrgyzstan", "central-asia"),
    ("Q863", "Tajikistan", "central-asia"),
    ("Q874", "Turkmenistan", "central-asia"),
    // Western Asia
    ("Q43", "Turkey", "western-asia"),
    ("Q12560", "Ottoman Empire", "western-asia"),
    ("Q801", "Israel", "western-asia"),
    ("Q219060", "State of Palestine", "western-asia"),
    ("Q192291", "Mandatory Palestine", "western-asia"),
    ("Q810", "Jordan", "western-asia"),
    ("Q822", "Lebanon", "western-asia"),
    ("Q858", "Syria", "western-asia"),
    ("Q796", "Iraq", "western-asia"),
    ("Q851", "Saudi Arabia", "western-asia"),
    ("Q878", "United Arab Emirates", "western-asia"),
    ("Q846", "Qatar", "western-asia"),
    ("Q398", "Bahrain", "western-asia"),
    ("Q817", "Kuwait", "western-asia"),
    ("Q842", "Oman", "western-asia"),
    ("Q805", "Yemen", "western-asia"),
    ("Q399", "Armenia", "western-asia"),
    ("Q227", "Azerbaijan", "western-asia"),
    ("Q230", "Georgia", "western-asia"),
    ("Q229", "Cyprus", "western-asia"),
    // Northern Africa
    ("Q79", "Egypt", "northern-africa"),
    ("Q262", "Algeria", "northern-africa"),
    ("Q1028", "Morocco", "northern-africa"),
    ("Q948", "Tunisia", "northern-africa"),
    ("Q1016", "Libya", "northern-africa"),
    ("Q1049", "Sudan", "northern-africa"),
    // Western Africa
    ("Q1033", "Nigeria", "western-africa"),
    ("Q117", "Ghana", "western-africa"),
    ("Q1041", "Senegal", "western-africa"),
    ("Q1008", "Ivory Coast", "western-africa"),
    ("Q965", "Burkina Faso", "western-africa"),
    ("Q912", "Mali", "western-africa"),
    ("Q1032", "Niger", "western-africa"),
    ("Q1006", "Guinea", "western-africa"),
    ("Q1007", "Guinea-Bissau", "western-africa"),
    ("Q1044", "Sierra Leone", "western-africa"),
    ("Q1014", "Liberia", "western-africa"),
    ("Q1005", "The Gambia", "western-africa"),
    ("Q1011", "Cape Verde", "western-africa"),
    ("Q945", "Togo", "western-africa"),
    ("Q962", "Benin", "western-africa"),
    ("Q1025", "Mauritania", "western-africa"),
    // Eastern Africa
    ("Q114", "Kenya", "eastern-africa"),
    ("Q1036", "Uganda", "eastern-africa"),
    ("Q924", "Tanzania", "eastern-africa"),
    ("Q115", "Ethiopia", "eastern-africa"),
    ("Q986", "Eritrea", "eastern-africa"),
    ("Q977", "Djibouti", "eastern-africa"),
    ("Q1045", "Somalia", "eastern-africa"),
    ("Q1037", "Rwanda", "eastern-africa"),
    ("Q967", "Burundi", "eastern-africa"),
    ("Q1027", "Mauritius", "eastern-africa"),
    ("Q1042", "Seychelles", "eastern-africa"),
    ("Q954", "Zimbabwe", "eastern-africa"),
    ("Q953", "Zambia", "eastern-africa"),
    ("Q1020", "Malawi", "eastern-africa"),
    ("Q1029", "Mozambique", "eastern-africa"),
    ("Q1019", "Madagascar", "eastern-africa"),
    ("Q970", "Comoros", "eastern-africa"),
    ("Q958", "South Sudan", "eastern-africa"),
    // Middle Africa
    ("Q974", "Democratic Republic of the Congo", "middle-africa"),
    ("Q971", "Republic of the Congo", "middle-africa"),
    ("Q1009", "Cameroon", "middle-africa"),
    ("Q929", "Central African Republic", "middle-africa"),
    ("Q657", "Chad", "middle-africa"),
    ("Q916", "Angola", "middle-africa"),
    ("Q983", "Equatorial Guinea", "middle-africa"),
    ("Q1000", "Gabon", "middle-africa"),
    ("Q1039", "São Tomé and Príncipe", "middle-africa"),
    // Southern Africa
    ("Q258", "South Africa", "southern-africa"),
    ("Q1030", "Namibia", "southern-africa"),
    ("Q963", "Botswana", "southern-africa"),
    ("Q1013", "Lesotho", "southern-africa"),
    ("Q1050", "Eswatini", "southern-africa"),
    // Oceania
    ("Q408", "Australia", "australia-and-new-zealand"),
    ("Q664", "New Zealand", "australia-and-new-zealand"),
    ("Q691", "Papua New Guinea", "melanesia"),
    ("Q712", "Fiji", "melanesia"),
    ("Q685", "Solomon Islands", "melanesia"),
    ("Q686", "Vanuatu", "melanesia"),
    ("Q33788", "New Caledonia", "melanesia"),
    ("Q710", "Kiribati", "micronesia"),
    ("Q695", "Palau", "micronesia"),
    ("Q702", "Federated States of Micronesia", "micronesia"),
    ("Q709", "Marshall Islands", "micronesia"),
    ("Q697", "Nauru", "micronesia"),
    ("Q16635", "Guam", "micronesia"),
    ("Q672", "Tuvalu", "polynesia"),
    ("Q683", "Samoa", "polynesia"),
    ("Q678", "Tonga", "polynesia"),
    ("Q16641", "American Samoa", "polynesia"),
    ("Q30971", "French Polynesia", "polynesia"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_countries_map_to_their_un_subregion() {
        for (qid, label, slug) in [
            ("Q30", "United States of America", "northern-america"),
            ("Q145", "United Kingdom", "northern-europe"),
            ("Q142", "France", "western-europe"),
            ("Q96", "Mexico", "central-america"),
            ("Q155", "Brazil", "south-america"),
            ("Q668", "India", "southern-asia"),
            ("Q148", "People's Republic of China", "eastern-asia"),
            ("Q252", "Indonesia", "south-eastern-asia"),
            ("Q43", "Turkey", "western-asia"),
            ("Q1033", "Nigeria", "western-africa"),
            ("Q79", "Egypt", "northern-africa"),
            ("Q408", "Australia", "australia-and-new-zealand"),
            ("Q865", "Taiwan", "eastern-asia"),
            ("Q1246", "Kosovo", "southern-europe"),
            ("Q22", "Scotland", "northern-europe"),
            ("Q1183", "Puerto Rico", "caribbean"),
        ] {
            assert_eq!(region_of_qid(qid), Some(slug), "{qid}");
            assert_eq!(region_of_label(label), Some(slug), "{label}");
        }
        assert_eq!(
            region_of_label("united states of america"),
            Some("northern-america")
        );
        assert_eq!(region_of_qid("Q2"), None);
        assert_eq!(region_of_label("Atlantis"), None);
    }

    #[test]
    fn historical_states_map_to_their_successor() {
        for (qid, slug) in [
            ("Q15180", "eastern-europe"),
            ("Q33946", "eastern-europe"),
            ("Q36704", "southern-europe"),
            ("Q16957", "western-europe"),
            ("Q28513", "western-europe"),
            ("Q34266", "eastern-europe"),
            ("Q43287", "western-europe"),
            ("Q172579", "southern-europe"),
            ("Q129286", "southern-asia"),
        ] {
            assert_eq!(region_of_qid(qid), Some(slug), "{qid}");
        }
    }

    #[test]
    fn every_table_slug_is_a_known_subregion_with_a_continent() {
        for (qid, label, slug) in COUNTRIES {
            let s = subregion(slug).unwrap_or_else(|| panic!("{qid} {label}: {slug}"));
            assert!(["europe", "americas", "asia", "africa", "oceania"].contains(&s.continent));
        }
        assert_eq!(SUBREGIONS.len(), 22);
        assert_eq!(
            subregion("northern-america").unwrap().label,
            "Northern America"
        );
        let mut qids: Vec<&str> = COUNTRIES.iter().map(|(q, _, _)| *q).collect();
        qids.sort();
        qids.dedup();
        assert_eq!(qids.len(), COUNTRIES.len(), "duplicate qid in the table");
    }
}
