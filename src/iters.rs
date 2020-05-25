use super::*;

#[derive(Clone, Debug)]
pub struct DevTreeReserveEntryIter<'a> {
    offset: usize,
    fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeReserveEntryIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_mem_rsvmap(),
            fdt,
        }
    }

    fn read(&self) -> Result<&'a fdt_reserve_entry, DevTreeError> {
        unsafe {
            // TODO alignment not guarunteed.
            if self.offset + size_of::<fdt_reserve_entry>() > self.fdt.buf.len() {
                Err(DevTreeError::InvalidLength)
            } else {
                Ok(transmute(self.fdt.buf.as_ptr().add(self.offset)))
            }
        }
    }
}

impl<'a> Iterator for DevTreeReserveEntryIter<'a> {
    type Item = &'a fdt_reserve_entry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset > self.fdt.totalsize() {
            None
        } else {
            let ret = self.read().unwrap();
            if ret.address == 0.into() && ret.size == 0.into() {
                return None;
            }

            self.offset += size_of::<fdt_reserve_entry>();
            Some(ret)
        }
    }
}

#[derive(Clone, Debug)]
pub struct DevTreeNodeIter<'a> {
    offset: usize,
    fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeNodeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            fdt,
        }
    }
}

impl<'a> Iterator for DevTreeNodeIter<'a> {
    type Item = DevTreeNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match step_parse_device_tree(self.offset, self.fdt) {
                Ok(ParsedItem::Prop(p)) => {
                    self.offset = p.new_offset;
                }
                Ok(ParsedItem::Node(n)) => {
                    self.offset = n.new_offset;
                    return Some(Self::Item {
                        fdt: self.fdt,
                        name: n.name.unwrap(),
                        prop_offset: n.new_offset,
                    })
                }
                Err(DevTreeError::Eof) => return None,
                Err(e) => panic!("Unexpected condition: {:?}", e),
            }
        }
    }
}

pub struct DevTreeNodePropIter<'a> {
    offset: usize,
    pub node: &'a DevTreeNode<'a>,
}

impl<'a> DevTreeNodePropIter<'a> {
    pub(crate) fn new(node: &'a DevTreeNode) -> Self {
        Self {
            offset: node.prop_offset, // FIXME Nee proprty offset from this
            node: node,
        }
    }
}

impl<'a> Iterator for DevTreeNodePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match step_parse_device_tree(self.offset, self.node.fdt) {
            Ok(ParsedItem::Prop(p)) => {
                self.offset = p.new_offset;

                Some(DevTreeProp {
                name: "todo - look up in string table",
                length: u32::from(p.header.len) as usize,
                node: self.node,
            })},
            Ok(ParsedItem::Node(_)) => {
                // If we hit a new node, we're done.
                None
            }
            Err(DevTreeError::Eof) => None,
            Err(e) => panic!("Unexpected condition: {:?}", e),
        }
    }
}

struct ParsedNode<'a> {
    /// Offset of the property value within the FDT buffer.
    new_offset: usize,
    name: Result<&'a str, Utf8Error>,
}
struct ParsedProp<'a> {
    new_offset: usize,
    /// Offset of the property value within the FDT buffer.
    value_offset: usize,
    header: &'a fdt_prop_header,
}

enum ParsedItem<'a> {
    Node(ParsedNode<'a>),
    Prop(ParsedProp<'a>),
}

// TODO Move into a DevTreeIter
fn step_parse_device_tree<'a>(
    mut offset: usize,
    fdt: &'a DevTree,
) -> Result<ParsedItem<'a>, DevTreeError> {
    unsafe {
        // Assert because we should end before the FDT_END occurs
        //
        // Since the intent of this library is to execute in a safety context, we might want to
        // just return None and perform this as a separate method.
        assert!(fdt.totalsize() > offset);
        loop {
            let fdt_val = fdt.buf.read_be_u32(offset)?;
            let fdt_tok = FromPrimitive::from_u32(fdt_val);
            offset += size_of::<u32>();

            match fdt_tok {
                Some(FdtTok::BeginNode) => {
                    let name = fdt.buf.read_bstring0(offset)?;

                    // Move to next u32 alignment after the str (including null byte).
                    offset += name.len() + 1;
                    // Align back to u32.
                    offset += fdt.buf.as_ptr().add(offset).align_offset(size_of::<u32>());

                    return Ok(ParsedItem::Node(ParsedNode {
                        name: core::str::from_utf8(name),
                        new_offset: offset,
                    }));
                }
                Some(FdtTok::Prop) => {
                    if offset + size_of::<fdt_reserve_entry>() > fdt.buf.len() {
                        panic!("");
                    }
                    let header = transmute::<*const u8, *const fdt_prop_header>(
                        fdt.buf.as_ptr().add(offset),
                    );
                    let prop_len = u32::from((*header).len);

                    offset += (prop_len as usize) + size_of::<fdt_prop_header>();
                    let value_offset = offset;

                    // Align back to u32.
                    offset += fdt
                        .buf
                        .as_ptr()
                        .add(offset)
                        .align_offset(size_of::<u32>());
                    return Ok(ParsedItem::Prop(ParsedProp {
                        header: &*header,
                        new_offset: offset,
                        value_offset,
                    }));
                }
                Some(FdtTok::EndNode) => {}
                Some(FdtTok::Nop) => {}
                Some(FdtTok::End) => {
                    return Err(DevTreeError::Eof);
                }
                None => {
                    panic!("Unknown FDT Token Value {:}", fdt_val);
                }
            }
        }
    }
}

