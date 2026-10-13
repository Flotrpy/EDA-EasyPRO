//! PCB layers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Layer {
    TopCopper,
    /// Inner copper layer, numbered from 1 below the top.
    Inner(u8),
    BottomCopper,
    TopSilk,
    BottomSilk,
    TopMask,
    BottomMask,
    TopPaste,
    BottomPaste,
    TopAssembly,
    BottomAssembly,
    Outline,
    Document,
    /// Through-hole items exist on every copper layer.
    Multi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Side {
    #[default]
    Top,
    Bottom,
}

impl Side {
    pub fn flip(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        }
    }
}

impl Layer {
    pub fn is_copper(self) -> bool {
        matches!(
            self,
            Layer::TopCopper | Layer::BottomCopper | Layer::Inner(_) | Layer::Multi
        )
    }

    pub fn name(self) -> String {
        match self {
            Layer::TopCopper => "Top Layer".into(),
            Layer::Inner(n) => format!("Inner {n}"),
            Layer::BottomCopper => "Bottom Layer".into(),
            Layer::TopSilk => "Top Silkscreen".into(),
            Layer::BottomSilk => "Bottom Silkscreen".into(),
            Layer::TopMask => "Top Solder Mask".into(),
            Layer::BottomMask => "Bottom Solder Mask".into(),
            Layer::TopPaste => "Top Paste".into(),
            Layer::BottomPaste => "Bottom Paste".into(),
            Layer::TopAssembly => "Top Assembly".into(),
            Layer::BottomAssembly => "Bottom Assembly".into(),
            Layer::Outline => "Board Outline".into(),
            Layer::Document => "Document".into(),
            Layer::Multi => "Multi-Layer".into(),
        }
    }

    /// Default display colour as `0xRRGGBB`, close to the conventions most
    /// PCB tools share so existing users feel at home.
    pub fn color(self) -> u32 {
        match self {
            Layer::TopCopper => 0xff3333,
            Layer::BottomCopper => 0x3366ff,
            Layer::Inner(1) => 0xc8a000,
            Layer::Inner(2) => 0x33aa55,
            Layer::Inner(3) => 0xaa44cc,
            Layer::Inner(_) => 0x44aaaa,
            Layer::TopSilk => 0xe8d000,
            Layer::BottomSilk => 0x88aa00,
            Layer::TopMask => 0x8a2be2,
            Layer::BottomMask => 0xaa6600,
            Layer::TopPaste => 0x808080,
            Layer::BottomPaste => 0x806060,
            Layer::TopAssembly => 0xb0b0b0,
            Layer::BottomAssembly => 0x707070,
            Layer::Outline => 0xff00ff,
            Layer::Document => 0xffffff,
            Layer::Multi => 0xc0c0c0,
        }
    }

    /// The equivalent layer on the opposite side of the board.
    pub fn flipped(self) -> Layer {
        match self {
            Layer::TopCopper => Layer::BottomCopper,
            Layer::BottomCopper => Layer::TopCopper,
            Layer::TopSilk => Layer::BottomSilk,
            Layer::BottomSilk => Layer::TopSilk,
            Layer::TopMask => Layer::BottomMask,
            Layer::BottomMask => Layer::TopMask,
            Layer::TopPaste => Layer::BottomPaste,
            Layer::BottomPaste => Layer::TopPaste,
            Layer::TopAssembly => Layer::BottomAssembly,
            Layer::BottomAssembly => Layer::TopAssembly,
            other => other,
        }
    }

    /// Copper layers of a board with `count` copper layers, top to bottom.
    pub fn copper_stack(count: u8) -> Vec<Layer> {
        let mut v = vec![Layer::TopCopper];
        for i in 1..count.saturating_sub(1) {
            v.push(Layer::Inner(i));
        }
        if count > 1 {
            v.push(Layer::BottomCopper);
        }
        v
    }

    /// Every layer shown in the layer panel for a board.
    pub fn all_for(count: u8) -> Vec<Layer> {
        let mut v = Layer::copper_stack(count);
        v.extend([
            Layer::TopSilk,
            Layer::BottomSilk,
            Layer::TopMask,
            Layer::BottomMask,
            Layer::TopPaste,
            Layer::BottomPaste,
            Layer::TopAssembly,
            Layer::BottomAssembly,
            Layer::Outline,
            Layer::Multi,
            Layer::Document,
        ]);
        v
    }

    /// Whether a copper item on `self` electrically exists on `other`.
    pub fn shares_copper(self, other: Layer) -> bool {
        self.is_copper()
            && other.is_copper()
            && (self == other || self == Layer::Multi || other == Layer::Multi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_sizes() {
        assert_eq!(Layer::copper_stack(1), vec![Layer::TopCopper]);
        assert_eq!(
            Layer::copper_stack(2),
            vec![Layer::TopCopper, Layer::BottomCopper]
        );
        assert_eq!(Layer::copper_stack(4).len(), 4);
        assert_eq!(Layer::copper_stack(4)[2], Layer::Inner(2));
    }

    #[test]
    fn flipping_is_symmetric() {
        for l in Layer::all_for(4) {
            assert_eq!(l.flipped().flipped(), l);
        }
        assert!(Layer::Multi.shares_copper(Layer::Inner(1)));
        assert!(!Layer::TopCopper.shares_copper(Layer::BottomCopper));
    }
}
