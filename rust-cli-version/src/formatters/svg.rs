use crate::models::{Sector, Volume};
use std::fmt::Write;

pub struct SvgGenerator {
    rows: usize,
    columns: usize,
    side: f64,
    factor: f64,
    height: f64,
    width: f64,
    name: String,
    show_islands: bool,
    island_jump: i64,
    island_min: usize,
    island_opacity: f64,
}

impl SvgGenerator {
    pub fn new(name: String) -> Self {
        let side = 40.0;
        let factor = 1.732; // sqrt(3) for hex height
        let columns = 32;  // 4 subsectors × 8 hexes wide
        let rows = 40;     // 4 subsectors × 10 hexes high

        SvgGenerator {
            rows,
            columns,
            side,
            factor,
            height: (side * factor * (rows as f64 + 0.5)).ceil(),
            width: (side * (columns as f64 * 1.5 + 0.5)).ceil(),
            name,
            show_islands: true,
            island_jump: 2,
            island_min: 2,
            island_opacity: 0.85,
        }
    }

    pub fn with_islands(mut self, show: bool, jump: i64, min: usize, opacity: f64) -> Self {
        self.show_islands = show;
        self.island_jump = jump;
        self.island_min = min;
        self.island_opacity = opacity;
        self
    }

    /// Convenience: generate with default island settings.
    pub fn generate_sector(sector: &Sector) -> String {
        SvgGenerator::new(sector.name.clone()).generate(sector)
    }

    pub fn generate(&self, sector: &Sector) -> String {
        let gen = self;
        let mut svg = String::new();

        svg.push_str(&gen.header());
        svg.push_str(&gen.tract_marks());
        svg.push_str(&gen.hex_grid());
        svg.push_str(&gen.islands(sector));
        svg.push_str(&gen.routes(sector));

        // Draw worlds
        for row in 0..sector.height {
            for col in 0..sector.width {
                if let Some(volume) = &sector.volumes[row][col] {
                    if !volume.is_empty() {
                        svg.push_str(&gen.world(volume, col + 1, row + 1));
                    }
                }
            }
        }

        svg.push_str(&gen.volume_numbers());
        svg.push_str(&gen.frame());
        svg.push_str("</svg>");

        svg
    }

    /// Outline clusters of nearby systems, reusing the shared Borders geometry.
    fn islands(&self, sector: &Sector) -> String {
        if !self.show_islands {
            return String::new();
        }
        let mut hexes: Vec<(i64, i64)> = Vec::new();
        for row in 0..sector.height {
            for col in 0..sector.width {
                if let Some(v) = &sector.volumes[row][col] {
                    if !v.is_empty() {
                        hexes.push(((col + 1) as i64, (row + 1) as i64));
                    }
                }
            }
        }
        let groups = crate::formatters::islands::borders(
            &hexes, self.side, self.factor, self.columns as i64, self.rows as i64,
            self.island_jump, self.island_min,
        );
        if groups.is_empty() {
            return String::new();
        }
        let mut out = String::from("<g class='islands'>\n");
        for g in &groups {
            let _ = write!(out, "  <g><!--island n={}-->", g.size);
            for ring in &g.loops {
                let pts: Vec<String> = ring.iter().map(|(x, y)| format!("{},{}", x, y)).collect();
                let _ = write!(
                    out,
                    "<polygon points='{}' stroke='{}' style='opacity:{}'/>",
                    pts.join(" "),
                    g.colour,
                    self.island_opacity
                );
            }
            out.push_str("</g>\n");
        }
        out.push_str("</g><!--/islands-->\n");
        out
    }

    /// Jump routes between systems — a port of Ruby build_routes / calc_route: per
    /// source, keep one line per slope bucket and never the reverse of an existing route.
    fn routes(&self, sector: &Sector) -> String {
        let mut present: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();
        let mut keys: Vec<(i64, i64)> = Vec::new();
        for row in 0..sector.height {
            for col in 0..sector.width {
                if let Some(v) = &sector.volumes[row][col] {
                    if !v.is_empty() {
                        let k = ((col + 1) as i64, (row + 1) as i64);
                        present.insert(k);
                        keys.push(k);
                    }
                }
            }
        }
        keys.sort_by_key(|&(c, r)| (c, r)); // by hex string "%02d%02d" (col-major)

        let mut routes: std::collections::HashMap<(i64, i64), Vec<(i64, i64)>> =
            std::collections::HashMap::new();
        let mut slopes: std::collections::HashMap<(i64, i64), Vec<f64>> =
            std::collections::HashMap::new();
        let mut out = String::from("<g class='routes'>\n");
        for &src in &keys {
            for tgt in each_hex_targets(src) {
                if !present.contains(&tgt) {
                    continue;
                }
                if let Some(line) = self.calc_route(src, tgt, &mut routes, &mut slopes) {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
        }
        out.push_str("</g>\n");
        out
    }

    fn calc_route(
        &self,
        src: (i64, i64),
        tgt: (i64, i64),
        routes: &mut std::collections::HashMap<(i64, i64), Vec<(i64, i64)>>,
        slopes: &mut std::collections::HashMap<(i64, i64), Vec<f64>>,
    ) -> Option<String> {
        let d = dist_hex(src, tgt);
        if d == 0 || d > 4 {
            return None; // self, or beyond jump-4, the widest class the stylesheet draws
        }
        if routes.get(&src).map_or(false, |v| v.contains(&tgt)) {
            return None; // already the reverse of an existing route
        }
        routes.entry(tgt).or_default().push(src);

        let (sx, sy) = self.center_of(src.0 as usize, src.1 as usize);
        let (dx, dy) = self.center_of(tgt.0 as usize, tgt.1 as usize);
        let src_pix = (sx as i64, sy as i64);
        let dst_pix = (dx as i64, dy as i64);
        let m = slope_pix(src_pix, dst_pix);
        if slopes.get(&src).map_or(false, |v| v.contains(&m)) {
            return None; // already a route on this slope
        }
        slopes.entry(src).or_default().push(m);

        Some(format!(
            "<!-- {:02}{:02}:{:02}{:02} --><line class='line{}' x1='{}' y1='{}' x2='{}' y2='{}' />",
            src.0, src.1, tgt.0, tgt.1, d, src_pix.0, src_pix.1, dst_pix.0, dst_pix.1
        ))
    }
    
    fn center_of(&self, col: usize, row: usize) -> (f64, f64) {
        let x = self.side + (col - 1) as f64 * self.side * 1.5;
        let y = (row - 1) as f64 * self.side * self.factor + 
                (self.side * self.factor / (1.0 + (col % 2) as f64));
        (x, y)
    }
    
    fn world(&self, volume: &Volume, col: usize, row: usize) -> String {
        let mut output = String::new();
        let (cx, cy) = self.center_of(col, row);
        
        if let Some(world) = &volume.world {
            write!(&mut output, "<!-- Volume: {:02}{:02} -->\n", col, row).unwrap();
            
            // Draw planet or belt
            if world.size == 0 {
                output.push_str(&self.draw_belt(cx, cy));
            } else {
                write!(&mut output, 
                    "    <circle class='planet' cx='{}' cy='{}' r='{}' />\n",
                    cx as i32, cy as i32, (self.side / 7.0) as i32
                ).unwrap();
            }
            
            // Starport
            write!(&mut output,
                "    <text class='spaceport' x='{}' y='{}'>{}</text>\n",
                cx as i32, (cy + self.side / 2.0) as i32, world.starport
            ).unwrap();
            
            // UWP
            write!(&mut output,
                "    <text x='{}' y='{}'>{}</text>\n",
                cx as i32, (cy + self.side / 1.3) as i32, world.uwp
            ).unwrap();
            
            // Name
            write!(&mut output,
                "    <text x='{}' y='{}'>{}</text>\n",
                cx as i32, (cy - self.side / 2.1) as i32, world.name
            ).unwrap();
            
            // Bases
            if world.bases_string().contains('N') {
                write!(&mut output,
                    "    <text class='symbol N' x='{}' y='{}'>⚓</text>\n",
                    (cx - self.side / 1.8) as i32, (cy - self.side / 6.0) as i32
                ).unwrap();
            }
            if world.bases_string().contains('S') {
                write!(&mut output,
                    "    <text class='symbol S' x='{}' y='{}'>⚜</text>\n",
                    (cx - self.side / 1.8) as i32, (cy + self.side / 2.4) as i32
                ).unwrap();
            }
            
            // Gas giant indicator
            if world.gas_giant {
                let gx = cx + self.side / 1.8;
                let gy = cy + self.side / 3.0;
                write!(&mut output,
                    "    <g class='gas-giant'>\n      <ellipse cx='{}' cy='{}' rx='{}' ry='{}' />\n      <circle cx='{}' cy='{}' r='{}' />\n    </g>\n",
                    gx as i32, gy as i32, 
                    (self.side / 6.5) as i32, (self.side / 13.0 * 0.3) as i32,
                    gx as i32, gy as i32, (self.side / 15.6) as i32
                ).unwrap();
            }
        }
        
        output
    }
    
    fn draw_belt(&self, cx: f64, cy: f64) -> String {
        let mut output = String::new();
        output.push_str("    <g class='belt'>\n");
        
        for i in 0..7 {
            let x = cx + ((i * 17 % (self.side / 3.0) as i32) as f64 - self.side / 6.0);
            let y = cy + ((i * 23 % (self.side / 3.0) as i32) as f64 - self.side / 6.0);
            write!(&mut output,
                "      <circle cx='{}' cy='{}' r='{}' />\n",
                x as i32, y as i32, (self.side / 15.0) as i32
            ).unwrap();
        }
        
        output.push_str("    </g>\n");
        output
    }
    
    fn tract_marks(&self) -> String {
        let mut output = String::new();
        output.push_str("<g class='tract'>\n");
        
        let letters = ["A", "B", "C", "D", "E", "F", "G", "H", 
                       "J", "K", "L", "M", "N", "O", "P", "R"];
        
        let subsector_height = 10.0 * self.side * self.factor;
        let subsector_width = 8.0 * self.side * 1.5;
        
        let mut idx = 0;
        for row in 0..4 {
            let y = row as f64 * subsector_height;
            for col in 0..4 {
                let x = col as f64 * subsector_width;
                
                if idx < letters.len() {
                    write!(&mut output,
                        "<rect x='{}' y='{}' width='{}' height='{}' />\n",
                        x as i32, y as i32, 
                        subsector_width as i32, subsector_height as i32
                    ).unwrap();
                    write!(&mut output,
                        "<text x='{}' y='{}'>{}</text>\n",
                        (x + 70.0) as i32, (y + 110.0) as i32, letters[idx]
                    ).unwrap();
                }
                idx += 1;
            }
        }
        
        output.push_str("</g>\n");
        write!(&mut output,
            "<text class='namestamp' x='30' y='{}'>{}</text>\n",
            (self.height - 40.0) as i32, self.name
        ).unwrap();
        
        output
    }
    
    fn volume_numbers(&self) -> String {
        let mut output = String::new();
        output.push_str("<g class='volumes'>\n");
        
        for row in 1..=self.rows {
            for col in 1..=self.columns {
                let x = self.side + (col - 1) as f64 * self.side * 1.5;
                let mut y = (row - 1) as f64 * self.side * self.factor + self.side * 0.2;
                if col % 2 == 0 {
                    y += self.side * self.factor / 2.0;
                }
                write!(&mut output,
                    "    <text x='{}' y='{}'>{:02}{:02}</text>\n",
                    x as i32, y as i32, col, row
                ).unwrap();
            }
        }
        
        output.push_str("</g>\n");
        output
    }
    
    fn hex_grid(&self) -> String {
        let mut output = String::new();
        output.push_str("<g class='grid'>\n");
        
        // Draw all hex rows - need rows*3+2 lines to get all the hex edges
        for j in 0..(self.rows * 3 + 2) {
            output.push_str(&self.hex_row(j / 2, j % 2 != 0));
            output.push('\n');
        }
        
        output.push_str("</g>\n");
        output
    }
    
    fn hex_row(&self, row: usize, top: bool) -> String {
        let side_h = self.side * self.factor / 2.0;
        let side_w = self.side / 2.0;
        let ly = row as f64 * 2.0 * side_h + side_h;
        
        let mut points = Vec::new();
        
        for j in 0..=(self.columns / 2) {
            let mut x = j as f64 * self.side * 3.0;
            let mut y = ly;
            points.push(format!("{},{}", x as i32, y as i32));
            
            x += side_w;
            if top { y -= side_h; } else { y += side_h; }
            points.push(format!("{},{}", x as i32, y as i32));
            
            x += self.side;
            points.push(format!("{},{}", x as i32, y as i32));
            
            x += side_w;
            if top { y += side_h; } else { y -= side_h; }
            points.push(format!("{},{}", x as i32, y as i32));
            
            x += self.side;
            points.push(format!("{},{}", x as i32, y as i32));
        }
        
        // Add final point for odd columns
        let x = (self.columns / 2) as f64 * self.side * 3.0 + self.side * 2.0 + side_w;
        let y = if top { ly - side_h } else { ly + side_h };
        points.push(format!("{},{}", x as i32, y as i32));
        
        format!("    <polyline points='{}' />", points.join(" "))
    }
    
    fn frame(&self) -> String {
        format!("    <polyline class='frame' points='0,0 {},0 {},{} 0,{} 0,0' />\n",
                self.width as i32, self.width as i32, self.height as i32, self.height as i32)
    }
    
    fn header(&self) -> String {
        format!(r#"<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"
  "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg width="{}px" height="{}px" version="1.1" xmlns="http://www.w3.org/2000/svg">
  <desc>{} Sector Map</desc>
  <style>
  text {{
    text-anchor: middle;
    font: 8px sans-serif;
  }}
  .tract text {{
    text-anchor: left;
    font: 120px sans-serif;
  }}
  text.namestamp {{
    text-anchor: start;
    font-size: 36px;
  }}
  text.symbol {{
    font-size: 14px;
  }}
  text.symbol.N {{
    font-size: 9px;
  }}
  g.volumes text {{
    opacity: 0.5;
  }}
  polyline {{
    stroke-width: 0.2;
    fill: none;
  }}
  circle {{
    stroke-width: 0.3;
  }}
  ellipse {{
    stroke-width: 0.3;
  }}
  .belt circle {{
    stroke: none;
    fill: black;
  }}
  .gas-giant ellipse, .gas-giant circle {{
    stroke-width: 0.3;
  }}
  .tract rect {{
    stroke-width: 2;
    fill: none;
    opacity: 0.2;
  }}
  .tract text {{
    opacity: 0.1;
    fill: black;
  }}
  .frame {{
    stroke-width: 2;
  }}
  g.islands polygon {{
    stroke-width: 6;
    fill: none;
    stroke-linejoin: round;
  }}
  line {{
    opacity: 0.3;
    stroke-linecap: round;
    stroke: #888;
  }}
  line.line1 {{ stroke-width: 4; }}
  line.line2 {{ stroke-width: 3; }}
  line.line3 {{ stroke-width: 2; stroke-dasharray: 5,5,1,5; opacity: 0.6; }}
  line.line4 {{ stroke-width: 1.5; stroke-dasharray: 2,6; }}

  /* Light mode (default) */
  svg {{
    background: #FAFAFA;
  }}
  text {{
    fill: #383A42;
  }}
  polyline, rect {{
    stroke: #ABB2BF;
    fill: none;
  }}
  circle {{
    fill: #121417;
    stroke: #FAFAFA;
  }}
  ellipse {{
    fill: none;
    stroke: #383A42;
  }}
  .planet {{
    fill: #121417;
    stroke: #121417;
    stroke-width: 1.5;
  }}
  .belt circle {{
    fill: #121417;
    stroke: none;
  }}
  .tract text {{
    fill: #ABB2BF;
  }}
  text.symbol {{
    fill: #121417;
  }}
  g.volumes text {{
    fill: #383A42;
    opacity: 0.5;
  }}
  .gas-giant circle {{
    fill: #383A42;
    stroke: #383A42;
  }}
  .gas-giant ellipse {{
    fill: none;
    stroke: #383A42;
  }}
  
  /* Dark mode */
  @media (prefers-color-scheme: dark) {{
    svg {{
      background: #121417;
    }}
    text {{
      fill: #ABB2BF;
    }}
    polyline, rect {{
      stroke: #ABB2BF;
      fill: none;
    }}
    circle {{
      fill: #ABB2BF;
      stroke: #121417;
    }}
    ellipse {{
      fill: none;
      stroke: #ABB2BF;
    }}
    .planet {{
      fill: #ABB2BF;
      stroke: #ABB2BF;
    }}
    .belt circle {{
      fill: #ABB2BF;
      stroke: none;
    }}
    .tract text {{
      fill: #FFF;
      opacity: 0.1;
    }}
    text.symbol {{
      fill: #ABB2BF;
    }}
    g.volumes text {{
      fill: #ABB2BF;
      opacity: 0.5;
    }}
    .gas-giant circle {{
      fill: #ABB2BF;
      stroke: #ABB2BF;
    }}
    .gas-giant ellipse {{
      fill: none;
      stroke: #ABB2BF;
    }}
  }}
  </style>
"#, self.width as i32, self.height as i32, self.name)
    }
}

/// Candidate route targets around a source, in Ruby each_hex order.
fn each_hex_targets(src: (i64, i64)) -> Vec<(i64, i64)> {
    let (x, y) = src;
    let ranges = [(-4, 4), (-4, 3), (-3, 3), (-3, 2), (-2, 2)];
    let mut out = Vec::new();
    for index in 0..5i64 {
        let (lo, hi) = ranges[index as usize];
        for i in [index, -index] {
            let x1 = x + i;
            for j in lo..=hi {
                out.push((x1, y + j));
            }
        }
    }
    out
}

/// Exact Traveller jump distance (hex-path count with the even-column stagger),
/// the same metric the island borders use. The former rounded straight-line
/// distance was off by one on staggered pairs.
fn dist_hex(a: (i64, i64), b: (i64, i64)) -> i64 {
    crate::models::sector::hex_jump(a.0, a.1, b.0, b.1)
}

fn floor_div(a: i64, b: i64) -> i64 {
    let q = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// Ruby Array#slope: floored integer slope of two pixel points, with its 0.01
/// (vertical) and 0.1 (rightward-flat) special cases, rounded to 0.1.
fn slope_pix(a: (i64, i64), b: (i64, i64)) -> f64 {
    let mut answer = if b.0 - a.0 == 0 {
        0.01
    } else {
        floor_div(b.1 - a.1, b.0 - a.0) as f64
    };
    if a.0 < b.0 && answer == 0.0 {
        answer = 0.1;
    }
    (answer * 10.0).round() / 10.0
}