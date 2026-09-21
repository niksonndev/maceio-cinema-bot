use crate::types::Cinema;

pub const DEFAULT_THEATER_ID: &str = "1162";

pub const CINEMAS: [Cinema; 3] = [
    Cinema {
        id: "1162",
        name: "Cinesystem",
        label: "Cinesystem (Parque Shopping Maceió)",
        url: "https://www.ingresso.com/cinema/cinesystem-maceio?city=maceio",
    },
    Cinema {
        id: "1230",
        name: "Centerplex",
        label: "Centerplex (Shopping Pátio Maceió)",
        url: "https://www.ingresso.com/cinema/centerplex-shopping-patio-maceio?city=maceio",
    },
    Cinema {
        id: "924",
        name: "Kinoplex",
        label: "Kinoplex (Maceió Shopping)",
        url: "https://www.ingresso.com/cinema/kinoplex-maceio?city=maceio",
    },
];

pub fn find_cinema_by_id(theater_id: &str) -> Option<&'static Cinema> {
    CINEMAS.iter().find(|c| c.id == theater_id)
}

pub fn cinema_list_text() -> String {
    let mut lines = vec![
        "🎬 Escolha o cinema (envie o número ou o nome):".to_string(),
        String::new(),
    ];
    for (i, c) in CINEMAS.iter().enumerate() {
        lines.push(format!("{}. {}", i + 1, c.label));
    }
    lines.join("\n")
}

pub fn parse_cinema_choice(text: &str) -> Option<&'static Cinema> {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<usize>() {
        if (1..=CINEMAS.len()).contains(&n) {
            return Some(&CINEMAS[n - 1]);
        }
    }
    CINEMAS
        .iter()
        .find(|c| t.contains(&c.name.to_lowercase()) || t.contains(&c.id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_number_and_name() {
        assert_eq!(parse_cinema_choice("1").unwrap().id, "1162");
        assert_eq!(parse_cinema_choice("3").unwrap().id, "924");
        assert_eq!(parse_cinema_choice("Kinoplex").unwrap().id, "924");
        assert!(parse_cinema_choice("xyz").is_none());
    }
}
