// most of the code here is inspired by the helix editor
use ratatui::prelude::*;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;

use crate::views::View;

slotmap::new_key_type! {
    pub struct ViewId;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Layout {
    Vertical,
    Horizontal,
}

struct Node {
    parent: ViewId,
    data: NodeData,
    area: Rect,
}

impl Node {
    fn view(view: Box<dyn View>) -> Node {
        Node {
            parent: ViewId::default(),
            data: NodeData::View(NodeViewData::new(view)),
            area: Rect::default(),
        }
    }

    fn container(layout: Layout) -> Node {
        Node {
            parent: ViewId::default(),
            data: NodeData::Container(NodeContainerData::new(layout)),
            area: Rect::default(),
        }
    }

    #[inline]
    fn is_view(&self) -> bool {
        matches!(self.data, NodeData::View(..))
    }

    #[inline]
    fn is_container(&self) -> bool {
        matches!(self.data, NodeData::Container(..))
    }
}

enum NodeData {
    View(NodeViewData),
    Container(NodeContainerData),
}

struct NodeViewData {
    view: Box<dyn View>,
}

impl NodeViewData {
    pub fn new(view: Box<dyn View>) -> Self {
        Self { view }
    }
}

struct NodeContainerData {
    childs: Vec<ViewId>,
    layout: Layout,
}

impl NodeContainerData {
    fn new(layout: Layout) -> Self {
        Self {
            childs: Vec::new(),
            layout,
        }
    }
}

/// the `Tree` type represents the parent child heirarchy for panels, parent nodes
/// are the containers, while the leaf nodes are the actual views which can also be another container,
///
/// the parent container assign to the leaf an area, with respect to the other leafs, if the leaf node is a view
/// it will just render its content to the view, if the leaf node is also a container, it will divide the given
/// area to its own leafs etc...
struct Tree {
    nodes: slotmap::HopSlotMap<ViewId, Node>,
    root: ViewId,
    focuse: ViewId,
}

impl Tree {
    pub fn new(area: Rect) -> Self {
        let mut nodes = slotmap::HopSlotMap::with_key();
        let root = Node::container(Layout::Vertical);
        let root = nodes.insert(root);
        nodes[root].parent = root;
        nodes[root].area = area;

        Tree {
            nodes,
            root,
            focuse: root,
        }
    }

    /// returns how many views the tree has opened
    fn views_count(&self) -> usize {
        self.nodes.values().filter(|node| node.is_view()).count()
    }

    fn push(&mut self, view: Box<dyn View>) -> ViewId {
        let parent = self.nodes[self.focuse].parent;
        let mut node = Node::view(view);
        node.parent = parent;

        let node = self.nodes.insert(node);

        match self.nodes[parent] {
            Node {
                area,
                data: NodeData::Container(ref mut ctr),
                ..
            } => {
                // the pushed view need to be inserted below/after current
                // focuse view, if there are no childs it means the index is `0`, if there
                // are childs, we look for the focused view position and add `1`
                let position = if ctr.childs.is_empty() {
                    0
                } else {
                    ctr.childs
                        .iter()
                        .position(|view_id| *view_id == self.focuse)
                        .unwrap()
                        + 1
                };
                ctr.childs.insert(position, node);
                self.recalculate(parent, area);
            }
            _ => panic!("unexpected, node parent is not a container"),
        };

        self.focuse = node;
        node
    }

    /// split the currently focused view into 2 with respect
    /// to the given layout
    fn split(&mut self, view: Box<dyn View>, layout: Layout) {
        let parent = self.nodes[self.focuse].parent;
        let node = self.nodes.insert(Node::view(view));

        let (ctr, area) = match self.nodes[parent] {
            Node {
                area,
                data: NodeData::Container(ref mut ctr),
                ..
            } => (ctr, area),
            _ => unreachable!(),
        };

        if ctr.layout == layout {
            let position = if ctr.childs.is_empty() {
                0
            } else {
                ctr.childs
                    .iter()
                    .position(|view_id| *view_id == self.focuse)
                    .unwrap()
                    + 1
            };
            ctr.childs.insert(position, node);
            self.nodes[node].parent = parent;
        } else {
            // if the layout is different from the current
            // container, we need to create a new container with the new layout
            // and assign it as a child for the current container, the view will be a child
            // of the new created container
            let mut split = Node::container(layout);
            split.parent = parent;

            let split = self.nodes.insert(split);
            self.nodes[self.focuse].parent = split;
            self.nodes[node].parent = split;

            let ctr = match &mut self.nodes[split] {
                Node {
                    data: NodeData::Container(ctr),
                    ..
                } => ctr,
                _ => unreachable!(),
            };

            ctr.childs.push(self.focuse);
            ctr.childs.push(node);

            let ctr = match &mut self.nodes[parent] {
                Node {
                    data: NodeData::Container(ctr),
                    ..
                } => ctr,
                _ => unreachable!(),
            };

            let position = ctr
                .childs
                .iter()
                .position(|view_id| *view_id == self.focuse)
                .unwrap();

            // replace the focuse child with the container
            ctr.childs[position] = split;
        };

        self.focuse = node;
        self.recalculate(parent, area);
    }

    /// remove the current focuse view
    fn remove(&mut self) {
        todo!();
    }

    #[inline]
    fn get_focuse(&self) -> &dyn View {
        match self.nodes[self.focuse].data {
            NodeData::View(NodeViewData { ref view, .. }) => view.as_ref(),
            _ => unreachable!(),
        }
    }

    #[inline]
    fn get_focuse_mut(&mut self) -> &mut dyn View {
        match self.nodes[self.focuse].data {
            NodeData::View(NodeViewData { ref mut view, .. }) => view.as_mut(),
            _ => unreachable!(),
        }
    }

    fn get_container_mut(&mut self) -> &mut NodeContainerData {
        let parent = self.nodes[self.focuse].parent;
        match &mut self.nodes[parent] {
            Node {
                data: NodeData::Container(ctr),
                ..
            } => ctr,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn resize(&mut self, size: (u16, u16)) {
        self.recalculate(self.root, Rect::new(0, 0, size.0, size.1));
    }

    /// recalculate areas from the given container node with the new area
    fn recalculate(&mut self, root: ViewId, area: Rect) {
        let mut stack = vec![(root, area)];

        while let Some((node, area_)) = stack.pop() {
            match &mut self.nodes[node] {
                Node {
                    area,
                    data: NodeData::Container(ctr),
                    ..
                } => {
                    *area = area_;

                    match ctr.layout {
                        Layout::Horizontal => {
                            let height = area.height / ctr.childs.len() as u16;
                            let mut offset = area.y;

                            for (i, child) in ctr.childs.iter().enumerate() {
                                let area = Rect::new(area.x, offset, area.width, height);
                                stack.push((*child, area));
                                offset += height;
                            }
                        }
                        Layout::Vertical => {
                            let width = area.width / ctr.childs.len() as u16;
                            let mut offset = area.x;

                            for (i, child) in ctr.childs.iter().enumerate() {
                                let area = Rect::new(offset, area.y, width, area.height);
                                stack.push((*child, area));
                                offset += width;
                            }
                        }
                    }
                }
                // for node view, we will just update the area
                // with the new calculated one by the parent
                Node {
                    area,
                    data: NodeData::View(view),
                    ..
                } => *area = area_,
            }
        }
    }
}

impl<'a> IntoIterator for &'a Tree {
    type IntoIter = TreeIter<'a>;
    type Item = <TreeIter<'a> as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        TreeIter {
            tree: self,
            stack: vec![self.root],
        }
    }
}

struct TreeIter<'a> {
    tree: &'a Tree,
    stack: Vec<ViewId>,
}

impl<'a> Iterator for TreeIter<'a> {
    type Item = (Rect, &'a NodeViewData);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let view = self.stack.pop()?;
            let node = &self.tree.nodes[view];

            match &node.data {
                NodeData::View(view) => return Some((node.area, view)),
                NodeData::Container(ctr) => {
                    self.stack.extend(&ctr.childs);
                }
            }
        }
    }
}

/// the compositor is responsible to display and render requested
/// views to the terminal
#[repr(transparent)]
pub struct Compositor {
    tree: Tree,
}

impl Compositor {
    #[inline(always)]
    pub fn new(area: Rect) -> Self {
        Self {
            tree: Tree::new(area),
        }
    }

    #[inline(always)]
    pub fn split_view(&mut self, view: Box<dyn View + Sync + 'static>, layout: Layout) {
        self.tree.split(view, layout);
    }

    /// resize the compositor viewport
    #[inline(always)]
    pub fn resize(&mut self, size: (u16, u16)) {
        self.tree.resize(size);
    }

    #[inline(always)]
    pub fn current_view(&self) -> &dyn View {
        self.tree.get_focuse()
    }

    #[inline(always)]
    pub fn current_view_mut(&mut self) -> &mut dyn View {
        self.tree.get_focuse_mut()
    }

    /// renders the views into the given buffer, compositor doesn't accept area because
    /// it will use whatever it has calculated in the tree
    #[inline(always)]
    pub fn render(&mut self, buffer: &mut Buffer) {
        for (area, view) in &self.tree {
            let block = Block::new().borders(Borders::RIGHT | Borders::TOP);
            let inner_area = block.inner(area);

            // TODO: show lines instead of blocks
            block.render(area, buffer);
            view.view.render(inner_area, buffer);
        }
    }
}
