use std::ops::{BitXor, BitXorAssign};
use std::sync::OnceLock;
use rand::random;
use strum::EnumCount;
use crate::bug::Bug;
use crate::hex::Hex;
use crate::hive::{Color, Hive, Tile};


const MIN_HEIGHT: usize = 0;
const MAX_HEIGHT: usize = 5;
const MIN_AXIS_VALUE: i32 = -21;
const MAX_AXIS_VALUE: i32 = 21;
const AXIS_ARRAY_SIZE: usize = (MAX_AXIS_VALUE - MIN_AXIS_VALUE) as usize;
const HEIGHT_ARRAY_SIZE: usize = MAX_HEIGHT - MIN_HEIGHT;
static ZOBRIST_TABLE: OnceLock<ZobristTable> = OnceLock::new();

#[derive(Copy, Clone, Default)]
pub struct ZobristHash(pub u64);

impl BitXor for ZobristHash {
    type Output = ZobristHash;

    fn bitxor(self, rhs: Self) -> Self::Output {
        ZobristHash(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for ZobristHash {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl ZobristHash {
    pub fn with_added_tile(&self, table: &ZobristTable, hex: &Hex, tile: &Tile) -> ZobristHash {
        *self ^ table.table_value(hex, tile)
    }

    pub fn with_removed_tile(&self, table: &ZobristTable, hex: &Hex, tile: &Tile) -> ZobristHash {
        *self ^ table.table_value(hex, tile)
    }

    pub fn with_turn_change(&self, table: &ZobristTable) -> ZobristHash {
        *self ^ table.black_to_move
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

type ZobristPieceTable =
    [[[[ZobristHash; AXIS_ARRAY_SIZE]; AXIS_ARRAY_SIZE]; HEIGHT_ARRAY_SIZE]; TILE_INDEX_COUNT];

pub struct ZobristTable {
    piece_table: Box<ZobristPieceTable>,
    pub black_to_move: ZobristHash,
}

impl ZobristTable {
    pub fn get() -> &'static ZobristTable {
        ZOBRIST_TABLE.get_or_init(ZobristTable::new)
    }

    fn new() -> ZobristTable {
        let mut piece_table: Box<ZobristPieceTable> = Box::new(
            [[[[ZobristHash(0); AXIS_ARRAY_SIZE]; AXIS_ARRAY_SIZE]; HEIGHT_ARRAY_SIZE]; TILE_INDEX_COUNT],
        );

        for tile_index in 0..TILE_INDEX_COUNT {
            for h in 0..HEIGHT_ARRAY_SIZE {
                for q in 0..AXIS_ARRAY_SIZE {
                    for r in 0..AXIS_ARRAY_SIZE {
                        piece_table[tile_index][h][q][r] = ZobristHash(random())
                    }
                }
            }
        }

        ZobristTable {
            piece_table,
            black_to_move: ZobristHash(random()),
        }
    }

    pub fn table_value(&self, hex: &Hex, tile: &Tile) -> ZobristHash {
        let tile_index: TileIndex = tile.into();
        let h_index = hex.h as usize;
        let q_index = if hex.q >= 0 {
            hex.q as usize + AXIS_ARRAY_SIZE / 2
        } else {
            hex.q.unsigned_abs() as usize
        };
        let r_index = if hex.r >= 0 {
            hex.r as usize + AXIS_ARRAY_SIZE / 2
        } else {
            hex.r.unsigned_abs() as usize
        };

        self.piece_table[tile_index][h_index][q_index][r_index]
    }

    pub fn hash(&self, hive: &Hive, active_player: Color) -> ZobristHash {
        let mut hash = ZobristHash(0);
        if active_player == Color::Black {
            hash ^= self.black_to_move;
        }
        for (hex, tile) in hive.map.iter() {
            let table_value = self.table_value(hex, tile);
            hash ^= table_value;
        }

        hash
    }
}

type TileIndex = usize;

const TILE_INDEX_COUNT: usize = Bug::COUNT * 2;

impl From<&Tile> for TileIndex {
    fn from(tile: &Tile) -> Self {
        let bug_index = tile.bug as usize;
        if tile.color == Color::Black {
            bug_index + Bug::COUNT
        } else {
            bug_index
        }
    }
}