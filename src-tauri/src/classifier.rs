use regex::Regex;

pub fn detect_script(text: &str) -> Option<&'static str> {
    let mut devanagari = 0;
    let mut gurmukhi = 0;
    let mut tamil = 0;
    let mut telugu = 0;
    let mut kannada = 0;
    let mut malayalam = 0;
    let mut bengali = 0;
    let mut hangul = 0;
    let mut kana = 0;
    let mut cjk_ideographs = 0;

    for c in text.chars() {
        let cp = c as u32;
        if (0x0900..=0x097F).contains(&cp) { devanagari += 1; }
        else if (0x0A00..=0x0A7F).contains(&cp) { gurmukhi += 1; }
        else if (0x0B80..=0x0BFF).contains(&cp) { tamil += 1; }
        else if (0x0C00..=0x0C7F).contains(&cp) { telugu += 1; }
        else if (0x0C80..=0x0CFF).contains(&cp) { kannada += 1; }
        else if (0x0D00..=0x0D7F).contains(&cp) { malayalam += 1; }
        else if (0x0980..=0x09FF).contains(&cp) { bengali += 1; }
        else if (0xAC00..=0xD7AF).contains(&cp) { hangul += 1; }
        else if (0x3040..=0x30FF).contains(&cp) { kana += 1; }
        else if (0x4E00..=0x9FFF).contains(&cp) { cjk_ideographs += 1; }
    }

    if devanagari >= 2 { return Some("Devanagari"); }
    if gurmukhi >= 2 { return Some("Gurmukhi"); }
    if tamil >= 2 { return Some("Tamil"); }
    if telugu >= 2 { return Some("Telugu"); }
    if kannada >= 2 { return Some("Kannada"); }
    if malayalam >= 2 { return Some("Malayalam"); }
    if bengali >= 2 { return Some("Bengali"); }
    if hangul >= 2 { return Some("Hangul"); }
    if kana >= 1 || (kana + cjk_ideographs >= 2 && kana > 0) { return Some("Japanese"); }
    if cjk_ideographs >= 2 { return Some("CJK_Ambiguous"); }

    None
}

/// 6-Tier Priority Language Classifier:
/// 1. Manual User Override Selection (if specified & not "Auto")
/// 2. Explicit yt-dlp Metadata Language (e.g. "hi", "pa", "ta", "te", "ko", "ja", "es")
/// 3. Strong Unicode Script Evidence (Devanagari, Gurmukhi, Tamil, etc.)
/// 4. Artist / Keyword Matcher (Regex)
/// 5. Title / Description Regional Signals
/// 6. Default Fallback ("English")
pub fn detect_language_advanced(
    title: &str,
    channel: &str,
    tags: &str,
    yt_lang: Option<&str>,
    user_override: &str,
) -> String {
    // Priority 1: Manual User Override Selection
    if !user_override.is_empty() && user_override.to_lowercase() != "auto" {
        return user_override.to_string();
    }

    // Priority 2: Explicit metadata language code from yt-dlp
    if let Some(code) = yt_lang {
        let code_clean = code.to_lowercase();
        if code_clean.starts_with("hi") { return "Hindi".into(); }
        if code_clean.starts_with("pa") { return "Punjabi".into(); }
        if code_clean.starts_with("ta") { return "Tamil".into(); }
        if code_clean.starts_with("te") { return "Telugu".into(); }
        if code_clean.starts_with("kn") { return "Kannada".into(); }
        if code_clean.starts_with("ml") { return "Malayalam".into(); }
        if code_clean.starts_with("bn") { return "Bengali".into(); }
        if code_clean.starts_with("mr") { return "Marathi".into(); }
        if code_clean.starts_with("ko") { return "Korean".into(); }
        if code_clean.starts_with("ja") { return "Japanese".into(); }
        if code_clean.starts_with("es") { return "Spanish".into(); }
        if code_clean.starts_with("fr") { return "French".into(); }
    }

    let combined = format!("{} {} {}", title, channel, tags).to_lowercase();

    // Check Instrumental signal
    let instrumental_re = Regex::new(r"(?i)\b(instrumental|karaoke|bgm|backing track|piano cover|guitar cover|flute version)\b").unwrap();
    if instrumental_re.is_match(&combined) {
        return "Instrumental".into();
    }

    let marathi_keywords = Regex::new(r"(?i)\b(marathi|saini|ajay-atool|natrang|maharashtra|lavani)\b").unwrap();

    // Priority 3: Strong Unicode Script Evidence
    if let Some(script) = detect_script(title) {
        match script {
            "Devanagari" => {
                if marathi_keywords.is_match(&combined) { return "Marathi".into(); }
                return "Hindi".into();
            },
            "Gurmukhi" => return "Punjabi".into(),
            "Tamil" => return "Tamil".into(),
            "Telugu" => return "Telugu".into(),
            "Kannada" => return "Kannada".into(),
            "Malayalam" => return "Malayalam".into(),
            "Bengali" => return "Bengali".into(),
            "Hangul" => return "Korean".into(),
            "Japanese" => return "Japanese".into(),
            "CJK_Ambiguous" => {
                if combined.contains("jpop") || combined.contains("anime") || combined.contains("japanese") {
                    return "Japanese".into();
                }
                return "Chinese/Japanese - ambiguous".into();
            },
            _ => {}
        }
    }

    // Priority 4 & 5: Artist & Regional Keyword Matcher
    let hindi_re = Regex::new(r"(?i)\b(hindi|bollywood|arijit|neha kakkar|jubin|shreya ghoshal|pritam|t-series|kesariya|brahmastra)\b").unwrap();
    let punjabi_re = Regex::new(r"(?i)\b(punjabi|punjab|sidhu|moosewala|diljit|karan aujla|ap dhillon|honey singh|guru randhawa)\b").unwrap();
    let tamil_re = Regex::new(r"(?i)\b(tamil|kollywood|anirudh|ar rahman|ilaiyaraaja|yuvan|jailer)\b").unwrap();
    let telugu_re = Regex::new(r"(?i)\b(telugu|tollywood|thaman|dsp|keeravani|rrr|naatu)\b").unwrap();
    let kannada_re = Regex::new(r"(?i)\b(kannada|sandalwood|charan|kgf|vishwaksen)\b").unwrap();
    let malayalam_re = Regex::new(r"(?i)\b(malayalam|mollywood|sushin shyam|hesam|minnal murali)\b").unwrap();
    let marathi_re = Regex::new(r"(?i)\b(marathi|saini|ajay-atool|natrang)\b").unwrap();
    let bengali_re = Regex::new(r"(?i)\b(bengali|tollywood-kolkata|jeetc|svf)\b").unwrap();
    let spanish_re = Regex::new(r"(?i)\b(spanish|español|reggaeton|latino|bad bunny|despacito|daddy yankee|karol g)\b").unwrap();
    let korean_re = Regex::new(r"(?i)\b(korean|kpop|k-pop|bts|blackpink|twice|stray kids)\b").unwrap();
    let japanese_re = Regex::new(r"(?i)\b(japanese|jpop|j-pop|anime ost|yoasobi|kenshi yonezu|ado|lisa)\b").unwrap();

    if hindi_re.is_match(&combined) { return "Hindi".into(); }
    if punjabi_re.is_match(&combined) { return "Punjabi".into(); }
    if tamil_re.is_match(&combined) { return "Tamil".into(); }
    if telugu_re.is_match(&combined) { return "Telugu".into(); }
    if kannada_re.is_match(&combined) { return "Kannada".into(); }
    if malayalam_re.is_match(&combined) { return "Malayalam".into(); }
    if marathi_re.is_match(&combined) { return "Marathi".into(); }
    if bengali_re.is_match(&combined) { return "Bengali".into(); }
    if spanish_re.is_match(&combined) { return "Spanish".into(); }
    if korean_re.is_match(&combined) { return "Korean".into(); }
    if japanese_re.is_match(&combined) { return "Japanese".into(); }

    // Priority 6: Default Fallback
    "English".into()
}

#[allow(dead_code)]
pub fn detect_language(title: &str, channel: &str, tags: &str, user_override: &str) -> String {
    detect_language_advanced(title, channel, tags, None, user_override)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_detection() {
        assert_eq!(detect_script("केसरिया - हिंदी"), Some("Devanagari"));
        assert_eq!(detect_script("੨੯੫ - ਪੰਜਾਬੀ"), Some("Gurmukhi"));
        assert_eq!(detect_script("தமிழ் பாடல்"), Some("Tamil"));
        assert_eq!(detect_script("తెలుగు పాట"), Some("Telugu"));
        assert_eq!(detect_script("방탄소년단"), Some("Hangul"));
        assert_eq!(detect_script("Shape of You"), None);
    }

    #[test]
    fn test_6_tier_priority_classification() {
        // Tier 1: User Manual Override
        assert_eq!(detect_language_advanced("Kesariya", "Arijit", "", Some("hi"), "Spanish"), "Spanish");

        // Tier 2: Explicit Metadata Language Code
        assert_eq!(detect_language_advanced("Random Song Title", "Channel Name", "", Some("hi"), "Auto"), "Hindi");
        assert_eq!(detect_language_advanced("Song Title", "Channel", "", Some("pa"), "Auto"), "Punjabi");
        assert_eq!(detect_language_advanced("Song Title", "Channel", "", Some("ja"), "Auto"), "Japanese");

        // Tier 3: Unicode Script Evidence (Overrules English keywords)
        assert_eq!(detect_language_advanced("केसरिया - Brahmastra", "Arijit", "", None, "Auto"), "Hindi");
        assert_eq!(detect_language_advanced("੨੯੫ Sidhu Moose Wala", "Sidhu", "", None, "Auto"), "Punjabi");

        // Tier 4: Artist & Keyword Matching
        assert_eq!(detect_language_advanced("Kesariya - Brahmastra", "Arijit Singh", "", None, "Auto"), "Hindi");
        assert_eq!(detect_language_advanced("295 Sidhu Moose Wala", "Sidhu", "", None, "Auto"), "Punjabi");
        assert_eq!(detect_language_advanced("Naatu Naatu RRR", "MM Keeravaani", "", None, "Auto"), "Telugu");
        assert_eq!(detect_language_advanced("Kaavaalaa Jailer", "Anirudh", "", None, "Auto"), "Tamil");
        assert_eq!(detect_language_advanced("Despacito ft. Daddy Yankee", "Daddy Yankee", "", None, "Auto"), "Spanish");
        assert_eq!(detect_language_advanced("Dynamite BTS", "HYBE", "", None, "Auto"), "Korean");

        // Instrumental Signal
        assert_eq!(detect_language_advanced("River Flows in You (Piano Cover BGM)", "Yiruma", "", None, "Auto"), "Instrumental");

        // Multilingual titles (Devanagari + English characters -> Hindi)
        assert_eq!(detect_language_advanced("Tum Hi Ho (तुम ही हो) Official Audio", "Arijit", "", None, "Auto"), "Hindi");

        // Tier 6: Default English Fallback
        assert_eq!(detect_language_advanced("Shape of You", "Ed Sheeran", "", None, "Auto"), "English");
        assert_eq!(detect_language_advanced("Unknown Track 123", "Unknown Channel", "", None, "Auto"), "English");
        assert_eq!(detect_language_advanced("", "", "", None, "Auto"), "English");
    }
}
