use crate::page::{PageHeader, PageType};
use nom::bytes::complete::take_while_m_n;
use nom::combinator::{cond, map, map_res};
use nom::error::VerboseError;
use nom::number::complete::{be_u16, be_u32, u8};
use nom::sequence::tuple;
use nom::IResult;
use std::num::NonZeroU16;

pub type Input<'a> = &'a [u8];
pub type PResult<'a, T> = IResult<Input<'a>, T, VerboseError<Input<'a>>>;

pub fn page_header<'a>(input: Input<'a>) -> PResult<'a, PageHeader> {
    let (input, page_type) = map_res(u8, PageType::try_from)(input)?;
    let interior = matches!(page_type, PageType::InteriorTable | PageType::InteriorIndex);
    let (input, (first_freeblock, cell_count, cell_content, fragmented_free_bytes, right_pointer)) =
        tuple((
            map(be_u16, NonZeroU16::new),
            be_u16,
            be_u16,
            u8,
            cond(interior, be_u32),
        ))(input)?;
    Ok((
        input,
        PageHeader {
            page_type,
            first_freeblock,
            cell_count,
            cell_content,
            fragmented_free_bytes,
            right_pointer,
        },
    ))
}

pub fn varint<'a>(input: Input<'a>) -> PResult<'a, u64> {
    let (input, (hibytes, last)) = tuple((take_while_m_n(0, 8, |b| b >= 0x80), u8))(input)?;
    let mut ans: u64 = 0;
    for b in hibytes {
        ans = (ans << 7) | (*b as u64 & 0x7f);
    }
    ans = (ans << if hibytes.len() == 8 { 8 } else { 7 }) | last as u64;
    Ok((input, ans))
}

#[test]
fn test_page_header() {
    let mut input = vec![
        0x0d_u8, // page_type is LeafTable
        0x00, 0x00, // no freeblocks
        0x00, 0x0a, // 10 cells
    ];
    // cell content starts at 4000
    input.extend_from_slice(4000u16.to_be_bytes().as_slice());
    // no fragmented bytes
    input.push(0);
    let expected = PageHeader {
        page_type: PageType::LeafTable,
        first_freeblock: None,
        cell_count: 10,
        cell_content: 4000,
        fragmented_free_bytes: 0,
        right_pointer: None,
    };
    let (_, header) = page_header(&input).unwrap();
    assert_eq!(header, expected);
}

#[test]
fn test_varint() {
    assert_eq!(varint([0x7f].as_slice()), Ok(([].as_slice(), 0x7f)));
    assert_eq!(
        varint([0b1_000_0001, 0b0_111_0000].as_slice()),
        Ok(([].as_slice(), 0b000_0001_111_0000))
    );
    assert_eq!(
        varint([0b1_111_1111, 0b1_000_0000, 0b1_111_0000, 0b0_000_0001].as_slice()),
        Ok(([].as_slice(), 0b1111111_0000000_1110000_0000001))
    );
    assert_eq!(
        varint([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff].as_slice()),
        Ok(([].as_slice(), 0xff_ff_ff_ff_ff_ff_ff_ff))
    );
}
