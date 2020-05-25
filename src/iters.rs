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

pub struct ParsedNode<'a> {
    /// Offset of the property value within the FDT buffer.
    new_offset: usize,
    name: Result<&'a str, DevTreeError>,
}
pub struct ParsedProp {
    new_offset: usize,
    /// Offset of the property value within the FDT buffer.
    propoff: usize,
    length: u32,
    nameoff: u32,
}

pub enum ParsedItem<'a> {
    Node(ParsedNode<'a>),
    Prop(ParsedProp),
}

// Static trait
impl<'a> ParsedItem<'a> {
    fn new_offset(&self) -> usize {
        use ParsedItem::*;
        match self {
            Prop(i) => i.new_offset,
            Node(i) => i.new_offset,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DevTreeParseIter<'a> {
    dt_offset: usize,
    fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeParseIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            dt_offset: fdt.off_dt_struct(),
            fdt,
        }
    }

    pub(crate) fn get_prop_str(&self, offset: usize) -> Result<&'a str, DevTreeError> {
        let str_offset = self.fdt.off_dt_strings() + offset;
        let name = self.fdt.buf.read_bstring0(str_offset)?;
        return core::str::from_utf8(name).or(Err(DevTreeError::Utf8Error));
    }
}

impl<'a> Iterator for DevTreeParseIter<'a> {
    type Item = ParsedItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = step_parse_device_tree(self.dt_offset, self.fdt);
        if res.is_ok() {
            let un = res.unwrap();
            self.dt_offset = un.new_offset();
            return Some(un);
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct DevTreeNodeIter<'a> {
    iter: DevTreeParseIter<'a>,
}

impl<'a> DevTreeNodeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            iter: DevTreeParseIter::new(fdt),
        }
    }
}

impl<'a> Iterator for DevTreeNodeIter<'a> {
    type Item = DevTreeNode<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(ParsedItem::Node(n)) => return Some(DevTreeNode::new(n.name, self.iter)),
                Some(_) => {
                    continue;
                }
                _ => return None,
            }
        }
    }
}

pub struct DevTreeNodePropIter<'a> {
    pub parse_iter: DevTreeParseIter<'a>,
}

impl<'a> DevTreeNodePropIter<'a> {
    pub(crate) fn new(node: &'a DevTreeNode) -> Self {
        Self {
            parse_iter: node.inner_iter,
        }
    }
}

impl<'a> Iterator for DevTreeNodePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let this = self.parse_iter;
            match self.parse_iter.next() {
                Some(ParsedItem::Prop(p)) => return Some(Self::Item {
                    iter: this,
                    nameoff: p.nameoff as usize,
                    length: p.length as usize,
                    propoff: p.propoff,
                }),
                // Return if a new node or an EOF.
                _ => return None,
            }
        }
    }
}

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
                        name: core::str::from_utf8(name).or(Err(DevTreeError::Utf8Error)),
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
                    let propoff = offset;

                    // Align back to u32.
                    offset += fdt.buf.as_ptr().add(offset).align_offset(size_of::<u32>());
                    return Ok(ParsedItem::Prop(ParsedProp {
                        new_offset: offset,
                        propoff,
                        length: prop_len,
                        nameoff: u32::from((*header).nameoff)
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
