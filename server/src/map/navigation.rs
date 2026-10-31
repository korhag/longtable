// Bounded grid navigation. Build collision once per request, then use A* over
// compact arrays. Diagonals cannot cut corners and height changes require stairs.
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use crate::map::layout::MapLayout;
use crate::map::{Segment, Vec2};

const EPS: f32 = 0.0001;
const MAX_CELLS: usize = 250_000;
const PROP: u8 = 1;
const WALL: u8 = 2;
type Cell = (i32, i32);

pub fn check_route(layout: &MapLayout, from: Vec2, to: Vec2) -> Result<(), String> {
    find_route(layout, from, to, &[], &[], [1, 1]).map(|_| ())
}

pub fn find_route(layout: &MapLayout, from: Vec2, to: Vec2, walls: &[Segment], blockers: &[Segment], footprint: [u32; 2]) -> Result<Vec<Cell>, String> {
    if ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite()) { return Err("choose a square on the map".into()); }
    let grid = Grid::new(layout, walls, blockers)?;
    let start = (from.x.floor() as i32, from.y.floor() as i32);
    let goal = (to.x.floor() as i32, to.y.floor() as i32);
    let mut search = Search { grid: &grid, footprint, stance: vec![-1; grid.count()] };
    search.standing(goal).map_err(str::to_string)?;
    if grid.index(start).is_none() { return Err("that square is off the map".into()); }
    if start == goal { return Ok(vec![start]); }
    // Preserve a useful reason if the destination is unreachable.
    let direct = crossed_cells(start, goal);
    let reason = direct.windows(2).find_map(|pair| search.step(pair[0], pair[1]).err()).unwrap_or("there is no walkable path to that square");
    search.route(start, goal).ok_or_else(|| reason.to_string())
}

struct Grid {
    width: i32, height: i32, levels: Vec<u8>, blocked: Vec<u8>, edges: Vec<u8>, stairs: HashSet<u64>,
}
impl Grid {
    fn new(layout: &MapLayout, walls: &[Segment], blockers: &[Segment]) -> Result<Self, String> {
        let count = (layout.width as usize).saturating_mul(layout.height as usize);
        if count == 0 || count > MAX_CELLS { return Err("this map is too large to find a walking route".into()); }
        let levels = match &layout.levels { Some(text) => crate::map::layout::decode_cells(text)?, None => vec![0; count] };
        if levels.len() != count { return Err("the height grid does not match the map".into()); }
        let mut grid = Self { width: layout.width as i32, height: layout.height as i32, levels, blocked: vec![0; count], edges: vec![0; count], stairs: HashSet::new() };
        for wall in walls { grid.wall(wall); }
        let mut polygon = Vec::new();
        for block in blockers {
            grid.paint_segment(block, PROP);
            if polygon.last().is_some_and(|last: &Vec2| !same(*last, block.a)) { polygon.clear(); }
            if polygon.is_empty() { polygon.push(block.a); }
            polygon.push(block.b);
            if polygon.len() >= 4 && same(polygon[0], block.b) { grid.fill_polygon(&polygon); polygon.clear(); }
        }
        for stair in layout.stairs.iter().flatten() {
            if let (Some(a), Some(b)) = (grid.index((stair.x as i32, stair.y as i32)), grid.index((stair.x2 as i32, stair.y2 as i32))) { grid.stairs.insert(grid.pair(a, b)); }
        }
        Ok(grid)
    }
    fn count(&self) -> usize { self.blocked.len() }
    fn index(&self, cell: Cell) -> Option<usize> {
        (cell.0 >= 0 && cell.1 >= 0 && cell.0 < self.width && cell.1 < self.height).then_some((cell.1 * self.width + cell.0).max(0) as usize)
    }
    fn cell(&self, index: usize) -> Cell { (index as i32 % self.width, index as i32 / self.width) }
    fn pair(&self, a: usize, b: usize) -> u64 { a.min(b) as u64 * self.count() as u64 + a.max(b) as u64 }
    fn level(&self, cell: Cell) -> u8 { self.index(cell).map(|i| self.levels[i]).unwrap_or(0) }
    fn edge(&self, from: Cell, to: Cell) -> bool {
        let bit = match (to.0 - from.0, to.1 - from.1) { (1, 0) => 1, (0, 1) => 2, (-1, 0) => 4, (0, -1) => 8, _ => 0 };
        self.index(from).is_some_and(|i| self.edges[i] & bit != 0)
    }
    fn wall(&mut self, segment: &Segment) {
        let a = segment.a; let b = segment.b;
        if (a.x-b.x).abs() < EPS && (a.x-a.x.round()).abs() < EPS {
            let x = a.x.round() as i32;
            for y in (a.y.min(b.y).floor() as i32).max(0)..(a.y.max(b.y).ceil() as i32).min(self.height) {
                if let Some(i) = self.index((x-1,y)) { self.edges[i] |= 1; }
                if let Some(i) = self.index((x,y)) { self.edges[i] |= 4; }
            }
        } else if (a.y-b.y).abs() < EPS && (a.y-a.y.round()).abs() < EPS {
            let y = a.y.round() as i32;
            for x in (a.x.min(b.x).floor() as i32).max(0)..(a.x.max(b.x).ceil() as i32).min(self.width) {
                if let Some(i) = self.index((x,y-1)) { self.edges[i] |= 2; }
                if let Some(i) = self.index((x,y)) { self.edges[i] |= 8; }
            }
        } else { self.paint_segment(segment, WALL); }
    }
    fn paint_segment(&mut self, segment: &Segment, bit: u8) {
        let a = segment.a; let b = segment.b;
        for y in (a.y.min(b.y).floor() as i32 - 1).max(0)..=(a.y.max(b.y).floor() as i32).min(self.height-1) {
            // Clip to one row first: a diagonal wall visits O(width + height) cells,
            // rather than testing every cell inside its bounding rectangle.
            let (left, right) = if (b.y-a.y).abs() < EPS { (a.x.min(b.x), a.x.max(b.x)) } else {
                let p=(y as f32+EPS-a.y)/(b.y-a.y); let q=(y as f32+1.-EPS-a.y)/(b.y-a.y);
                let lo=p.min(q).max(0.); let hi=p.max(q).min(1.);
                if lo>hi {continue;}
                let x0=a.x+(b.x-a.x)*lo; let x1=a.x+(b.x-a.x)*hi;
                (x0.min(x1),x0.max(x1))
            };
            for x in (left.floor() as i32 - 1).max(0)..=(right.floor() as i32).min(self.width-1) {
                if segment_in_cell(a,b,x,y) { let i = self.index((x,y)).unwrap(); self.blocked[i] |= bit; }
            }
        }
    }
    fn fill_polygon(&mut self, polygon: &[Vec2]) {
        let min_y = polygon.iter().map(|p| p.y).fold(f32::INFINITY, f32::min).floor() as i32;
        let max_y = polygon.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
        for y in min_y.max(0)..max_y.min(self.height) {
            let scan = y as f32 + 0.5;
            let mut cuts = Vec::new();
            for pair in polygon.windows(2) {
                let (a,b)=(pair[0],pair[1]);
                if (a.y > scan) != (b.y > scan) { cuts.push(a.x + (scan-a.y)*(b.x-a.x)/(b.y-a.y)); }
            }
            cuts.sort_by(f32::total_cmp);
            for pair in cuts.chunks_exact(2) {
                let start = ((pair[0]-0.5+EPS).ceil() as i32).max(0);
                let end = ((pair[1]-0.5-EPS).floor() as i32).min(self.width-1);
                for x in start..=end { let i=self.index((x,y)).unwrap(); self.blocked[i] |= PROP; }
            }
        }
    }
}

fn same(a: Vec2,b: Vec2) -> bool { (a.x-b.x).abs()<EPS && (a.y-b.y).abs()<EPS }
fn segment_in_cell(a: Vec2,b: Vec2,x:i32,y:i32) -> bool {
    let mut lo: f32=0.; let mut hi: f32=1.;
    for (start,delta,min,max) in [(a.x,b.x-a.x,x as f32+EPS,x as f32+1.-EPS),(a.y,b.y-a.y,y as f32+EPS,y as f32+1.-EPS)] {
        if delta.abs()<EPS { if start<min || start>max {return false;} }
        else { let p=(min-start)/delta; let q=(max-start)/delta; lo=lo.max(p.min(q)); hi=hi.min(p.max(q)); if lo>hi {return false;} }
    }
    true
}

struct Search<'a> { grid: &'a Grid, footprint: [u32;2], stance: Vec<i16> }
impl Search<'_> {
    fn body(&self, cell:Cell) -> (i32,i32,i32,i32) {
        let w=self.footprint[0].max(1) as i32; let h=self.footprint[1].max(1) as i32;
        (cell.0-w/2,cell.1-h/2,w,h)
    }
    fn standing(&mut self, cell:Cell) -> Result<u8,&'static str> {
        let Some(index)=self.grid.index(cell) else {return Err("that square is off the map")};
        if self.stance[index]>=0 {return decode_stance(self.stance[index]);}
        let (x,y,w,h)=self.body(cell);
        let mut result=Ok(self.grid.level(cell));
        'body: for cy in y..y+h { for cx in x..x+w {
            let Some(i)=self.grid.index((cx,cy)) else { result=Err("that square is off the map");break 'body;};
            if self.grid.blocked[i]&PROP!=0 {result=Err("something is in the way");break 'body;}
            if self.grid.blocked[i]&WALL!=0 || (cx>x && self.grid.edge((cx-1,cy),(cx,cy))) || (cy>y && self.grid.edge((cx,cy-1),(cx,cy))) {result=Err("a wall is in the way");break 'body;}
            if self.grid.levels[i]!=self.grid.level(cell) {result=Err("that body does not fit on one level");break 'body;}
        }}
        self.stance[index]=encode_stance(result);
        result
    }
    fn cardinal(&mut self, from:Cell,to:Cell) -> Result<(),&'static str> {
        let there=self.standing(to)?;
        let here=self.grid.level(from);
        let diff=here.abs_diff(there);
        if diff>1 {return Err("that drop is more than one step")}
        let (x,y,w,h)=self.body(to); let dx=to.0-from.0;let dy=to.1-from.1;
        for cy in y..y+h {for cx in x..x+w {
            // Only the leading row/column enters new ground.
            if (dx==1 && cx!=x+w-1)||(dx==-1 && cx!=x)||(dy==1 && cy!=y+h-1)||(dy==-1 && cy!=y) {continue;}
            let prev=(cx-dx,cy-dy);let next=(cx,cy);
            if self.grid.edge(prev,next) {return Err("a wall is in the way")}
            if diff==1 {
                let Some(a)=self.grid.index(prev) else {return Err("that square is off the map")};
                let b=self.grid.index(next).unwrap();
                if !self.grid.stairs.contains(&self.grid.pair(a,b)) {return Err("there are no stairs on that edge")}
            }
        }}
        Ok(())
    }
    fn step(&mut self, from:Cell,to:Cell) -> Result<(),&'static str> {
        if from.0==to.0 || from.1==to.1 {return self.cardinal(from,to);}
        let across=(to.0,from.1);let down=(from.0,to.1);
        if [across,down,to].iter().any(|&p| self.grid.level(p)!=self.grid.level(from)) {return Err("there are no stairs on that edge")}
        self.cardinal(from,across)?;self.cardinal(from,down)?;self.cardinal(across,to)?;self.cardinal(down,to)
    }
    fn route(&mut self,start:Cell,goal:Cell)->Option<Vec<Cell>> {
        let start_i=self.grid.index(start)?;let goal_i=self.grid.index(goal)?;
        let mut distance=vec![u32::MAX;self.grid.count()];let mut previous=vec![usize::MAX;self.grid.count()];let mut closed=vec![false;self.grid.count()];
        let mut heap=BinaryHeap::new();let mut serial=0_u32;
        let h=heuristic(start,goal);distance[start_i]=0;heap.push(Reverse((h,h,serial,start_i)));
        while let Some(Reverse((_,_,_,index)))=heap.pop() {
            if closed[index] {continue;}closed[index]=true;
            let from=self.grid.cell(index);
            if index==goal_i {
                let mut path=vec![goal];let mut cursor=index;
                while cursor!=start_i {cursor=previous[cursor];path.push(self.grid.cell(cursor));}
                path.reverse();return Some(path);
            }
            for (dx,dy) in [(1,0),(0,1),(-1,0),(0,-1),(1,1),(-1,1),(-1,-1),(1,-1)] {
                let next=(from.0+dx,from.1+dy);let Some(ni)=self.grid.index(next) else {continue;};
                if closed[ni] || self.step(from,next).is_err() {continue;}
                let cost=distance[index]+if dx!=0&&dy!=0 {14} else {10};
                if cost>=distance[ni] {continue;}
                distance[ni]=cost;previous[ni]=index;serial+=1;let h=heuristic(next,goal);
                heap.push(Reverse((cost+h,h,serial,ni)));
            }
        }
        None
    }
}
fn heuristic(a:Cell,b:Cell)->u32 {let dx=a.0.abs_diff(b.0);let dy=a.1.abs_diff(b.1);10*dx.max(dy)+4*dx.min(dy)}
fn encode_stance(result:Result<u8,&str>)->i16 {match result {Ok(level)=>level as i16,Err("that square is off the map")=>10,Err("something is in the way")=>11,Err("a wall is in the way")=>12,Err(_)=>13}}
fn decode_stance(value:i16)->Result<u8,&'static str> {match value {10=>Err("that square is off the map"),11=>Err("something is in the way"),12=>Err("a wall is in the way"),13=>Err("that body does not fit on one level"),_=>Ok(value as u8)}}
fn crossed_cells(start:Cell,goal:Cell)->Vec<Cell> {let(mut x,mut y)=start;let mut cells=vec![start];while (x,y)!=goal {if x!=goal.0{x+=(goal.0-x).signum();cells.push((x,y));}if y!=goal.1{y+=(goal.1-y).signum();cells.push((x,y));}}cells}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::layout::{encode_cells, StairLink};

    fn raised() -> MapLayout {
        let mut levels = vec![0_u8; 36];
        levels[1] = 1;
        levels[2] = 2;
        levels[6] = 1;
        let mut layout = MapLayout::blank(6, 6, "core/grass");
        layout.levels = Some(encode_cells(&levels));
        layout.stairs = Some(vec![
            StairLink { id: "up".into(), asset: "steps/stair".into(), x: 0, y: 0, x2: 1, y2: 0 },
            StairLink { id: "top".into(), asset: "steps/stair".into(), x: 1, y: 0, x2: 2, y2: 0 },
        ]);
        layout
    }

    #[test]
    fn stairs_climb_and_a_cliff_does_not() {
        let layout = raised();
        layout.validate().unwrap();
        check_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(1.5, 0.5)).unwrap();
        check_route(&layout, Vec2::new(1.5, 0.5), Vec2::new(2.5, 0.5)).unwrap();
        let cliff = check_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(0.5, 1.5)).unwrap_err();
        assert!(cliff.contains("no stairs"), "{cliff}");
        let drop = check_route(&layout, Vec2::new(2.5, 0.5), Vec2::new(3.5, 0.5)).unwrap_err();
        assert!(drop.contains("more than one step"), "{drop}");
    }

    #[test]
    fn a_stair_on_flat_ground_is_refused() {
        let mut layout = MapLayout::blank(6, 6, "core/grass");
        layout.stairs = Some(vec![
            StairLink { id: "flat".into(), asset: "steps/stair".into(), x: 0, y: 0, x2: 1, y2: 0 },
            StairLink { id: "far".into(), asset: "steps/stair".into(), x: 3, y: 2, x2: 3, y2: 3 },
        ]);
        let err = layout.validate().unwrap_err();
        assert!(err.contains("one step apart"), "{err}");
        assert!(err.contains("(0, 0) to (1, 0)"), "{err}");
        assert!(err.contains("(3, 2) to (3, 3)"), "{err}");
    }

    #[test]
    fn a_walk_goes_around_a_wall() {
        let layout = MapLayout::blank(4, 4, "core/grass");
        let wall = Segment { a: Vec2::new(1.0, 0.0), b: Vec2::new(1.0, 1.0) };
        let path = find_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(1.5, 0.5), &[wall], &[], [1, 1]).unwrap();
        assert_eq!(path, vec![(0, 0), (0, 1), (1, 1), (1, 0)]);
        let blocked = Segment { a: Vec2::new(1.0, 0.0), b: Vec2::new(1.0, 4.0) };
        let err = find_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(1.5, 0.5), &[blocked], &[], [1, 1]).unwrap_err();
        assert!(err.contains("wall"), "{err}");
    }
    fn rectangle(x:f32,y:f32,w:f32,h:f32)->Vec<Segment> {
        let points=[Vec2::new(x,y),Vec2::new(x+w,y),Vec2::new(x+w,y+h),Vec2::new(x,y+h)];
        (0..4).map(|i|Segment{a:points[i],b:points[(i+1)%4]}).collect()
    }
    #[test]
    fn small_corner_props_block_the_body_and_large_props_block_their_interior() {
        let map=MapLayout::blank(8,8,"grass");let trunk=rectangle(2.8,2.8,0.4,0.4);
        let route=find_route(&map,Vec2::new(0.5,2.5),Vec2::new(5.5,2.5),&[],&trunk,[1,1]).unwrap();
        assert!(route.iter().all(|&(x,y)|!(x>=2&&x<4&&y>=2&&y<4)),"the whole body clears the trunk");
        assert!(find_route(&map,Vec2::new(0.5,2.5),Vec2::new(3.5,3.5),&[],&trunk,[1,1]).is_err());
        assert!(find_route(&map,Vec2::new(0.5,2.5),Vec2::new(3.5,3.5),&[],&rectangle(2.,2.,4.,4.),[1,1]).is_err());
    }
    #[test]
    fn a_wide_figure_cannot_squeeze_through_a_one_square_passage() {
        let map=MapLayout::blank(10,10,"grass");let mut blocks=rectangle(4.,0.,1.,4.);blocks.extend(rectangle(4.,5.,1.,5.));
        assert!(find_route(&map,Vec2::new(2.,5.),Vec2::new(8.,5.),&[],&blocks,[2,2]).is_err());
        find_route(&map,Vec2::new(2.5,4.5),Vec2::new(8.5,4.5),&[],&blocks,[1,1]).unwrap();
    }
    #[test]
    fn stairs_are_found_away_from_the_direct_line_without_diagonal_corner_cutting() {
        let mut map=MapLayout::blank(6,6,"grass");let mut levels=vec![0;36];
        for y in 0..6 {for x in 3..6 {levels[y*6+x]=1;}}
        map.levels=Some(encode_cells(&levels));map.stairs=Some(vec![StairLink{id:"up".into(),asset:"stairs".into(),x:2,y:0,x2:3,y2:0}]);
        let route=find_route(&map,Vec2::new(0.5,2.5),Vec2::new(4.5,2.5),&[],&[],[1,1]).unwrap();
        assert!(route.windows(2).any(|p|p==[(2,0),(3,0)]));
        let wall=Segment{a:Vec2::new(1.,0.),b:Vec2::new(1.,1.)};
        let corner=find_route(&MapLayout::blank(4,4,"grass"),Vec2::new(0.5,0.5),Vec2::new(1.5,1.5),&[wall],&[],[1,1]).unwrap();
        assert_eq!(corner,vec![(0,0),(0,1),(1,1)]);
    }
    #[test]
    fn the_largest_built_map_uses_a_short_diagonal_route() {
        let route=find_route(&MapLayout::blank(200,200,"grass"),Vec2::new(0.5,0.5),Vec2::new(199.5,199.5),&[],&[],[1,1]).unwrap();
        assert_eq!(route.len(),200);
    }

}
