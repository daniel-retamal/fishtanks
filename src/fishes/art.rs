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

pub static GOTA: Figure = Figure {
    left: &[Art {
        lines: &["<(°_°)~"],
        paint: &["00e0e00"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &["~(°_°)>"],
        paint: &["00e0e00"],
        body_row: 0,
    }],
    zooming: None,
};

pub static HACHA: Figure = Figure {
    left: &[Art {
        lines: &["  /|", "<°|><", " '\"'"],
        paint: &["  00", "0e000", " 111"],
        body_row: 1,
    }],
    right: &[Art {
        lines: &[" |\\", "><|°>", " '\"'"],
        paint: &[" 00", "000e0", " 111"],
        body_row: 1,
    }],
    zooming: None,
};

pub static BARRIL: Figure = Figure {
    left: &[Art {
        lines: &["(oo)(((((><"],
        paint: &["1ee10000000"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &["><)))))(oo)"],
        paint: &["00000001ee1"],
        body_row: 0,
    }],
    zooming: None,
};

pub static OGRO: Figure = Figure {
    left: &[Art {
        lines: &["vVv°((<>"],
        paint: &["111e0000"],
        body_row: 0,
    }],
    right: &[Art {
        lines: &["><))°vVv"],
        paint: &["0000e111"],
        body_row: 0,
    }],
    zooming: None,
};

pub static QUIMERA: Figure = Figure {
    left: &[Art {
        lines: &["      |\\", "<°((]{(<>~~~", "     |/"],
        paint: &["      11", "0e0011000222", "     11"],
        body_row: 1,
    }],
    right: &[Art {
        lines: &["    /|", "~~~><)}]))°>", "     \\|"],
        paint: &["    11", "2220001100e0", "     11"],
        body_row: 1,
    }],
    zooming: None,
};

pub static CELACANTO: Figure = Figure {
    left: &[Art {
        lines: &["     ,/\\,", "<°(%%%%(<>{~", "     '\\/'"],
        paint: &["     2222", "0e0111100022", "     2222"],
        body_row: 1,
    }],
    right: &[Art {
        lines: &["   ,/\\,", "~}><)%%%%)°>", "   '\\/'"],
        paint: &["   2222", "2200011110e0", "   2222"],
        body_row: 1,
    }],
    zooming: None,
};
