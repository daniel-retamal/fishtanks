use super::figure::{Art, Figure};

pub static SNAILFISH: Figure = Figure {
    left: &[Art {
        lines: &["º._(@)"],
        paint: &["e00121"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &["(@)_.º"],
        paint: &["12100e"],
        body_row: 0,
    }],
    zooming: Some((
        Art {
            lines: &["(@)"],
            paint: &["121"],
            body_row: 0,
        },
        Art {
            lines: &["(@)"],
            paint: &["121"],
            body_row: 0,
        },
    )),
};

pub static CRABFISH: Figure = Figure {
    left: &[Art {
        lines: &["(\\/ºº(##)"],
        paint: &["000ee2jj2"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &["(##)ºº\\/)"],
        paint: &["2jj2ee000"],
        body_row: 0,
    }],
    zooming: None,
};

pub static BOXFISH: Figure = Figure {
    left: &[Art {
        lines: &["<º[ooo]<"],
        paint: &["0e000000"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &[">[ooo]º>"],
        paint: &["000000e0"],
        body_row: 0,
    }],
    zooming: None,
};

pub static SEAHORSEFISH: Figure = Figure {
    left: &[Art {
        lines: &["<º}", " (", " ɔ"],
        paint: &["1e0", " 0", " 2"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &["{º>", " )", " c"],
        paint: &["0e1", " 0", " 2"],
        body_row: 0,
    }],
    zooming: None,
};

pub static OCTOPUSFISH: Figure = Figure {
    left: &[
        Art {
            lines: &[" ,-.", "(ººo)", " )))"],
            paint: &[" 000", "0ee00", " 000"],
            body_row: 1,
        },
        Art {
            lines: &[" ,-.", "(ººo)", " ((("],
            paint: &[" 000", "0ee00", " 000"],
            body_row: 1,
        },
    ],
    right: &[
        Art {
            lines: &[" .-,", "(oºº)", " ((("],
            paint: &[" 000", "00ee0", " 000"],
            body_row: 1,
        },
        Art {
            lines: &[" .-,", "(oºº)", " )))"],
            paint: &[" 000", "00ee0", " 000"],
            body_row: 1,
        },
    ],
    zooming: None,
};

pub static SHOALFISH: Figure = Figure {
    left: &[
        Art {
            lines: &[" <=   <=", "<=  <=  <="],
            paint: &[" 01   01", "01  01  01"],
            body_row: 1,
        },
        Art {
            lines: &["  <= <=", "<=   <= <="],
            paint: &["  01 01", "01   01 01"],
            body_row: 1,
        },
    ],
    right: &[
        Art {
            lines: &["  =>   =>", "=>  =>  =>"],
            paint: &["  10   10", "10  10  10"],
            body_row: 1,
        },
        Art {
            lines: &["   => =>", "=> =>   =>"],
            paint: &["   10 10", "10 10   10"],
            body_row: 1,
        },
    ],
    zooming: Some((
        Art {
            lines: &["<= <= <= <= <="],
            paint: &["01 01 01 01 01"],
            body_row: 0,
        },
        Art {
            lines: &["=> => => => =>"],
            paint: &["10 10 10 10 10"],
            body_row: 0,
        },
    )),
};

pub static HAMMERFISH: Figure = Figure {
    left: &[Art {
        lines: &["º", "]((((><", "º"],
        paint: &["e", "0120120", "e"],
        body_row: 1,
    }],
    right: &[Art {
        lines: &["      º", "<>))))[", "      º"],
        paint: &["      e", "0210210", "      e"],
        body_row: 1,
    }],
    zooming: None,
};
