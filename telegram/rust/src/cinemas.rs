use crate::types::Cinema;

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
    CINEMAS.iter().find(|cinema| cinema.id == theater_id)
}
