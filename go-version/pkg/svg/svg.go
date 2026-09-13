package svg

import (
	"astromapper/pkg/models"
	"fmt"
	"math"
	"sort"
	"strings"
)

type SVGGenerator struct {
	Rows    int
	Columns int
	Side    float64
	Factor  float64
	Height  int
	Width   int
	Mark    float64
	Name    string
	Volumes []*models.Volume
	Routes  map[string][]string
	Slopes  map[string][]float64

	// Island borders (clusters of nearby systems outlined on the map).
	ShowIslands   bool
	IslandJump    int
	IslandMin     int
	IslandOpacity float64
}

func NewSVGGenerator(name string) *SVGGenerator {
	side := 40.0
	factor := 1.732 // Increased by 1/6th (multiply by 7/6)
	// Standard Traveller sector: 4×4 subsectors, each 8×10 hexes
	columns := 32 // 4 subsectors × 8 hexes wide
	rows := 40    // 4 subsectors × 10 hexes high

	return &SVGGenerator{
		Rows:    rows,
		Columns: columns,
		Side:    side,
		Factor:  factor,
		Height:  int(math.Ceil(side * factor * (float64(rows) + 0.5))),
		Width:   int(math.Ceil(side * (float64(columns)*1.5 + 0.5))),
		Mark:    13,
		Name:    name,
		Routes:  make(map[string][]string),
		Slopes:  make(map[string][]float64),

		ShowIslands:   true,
		IslandJump:    2,
		IslandMin:     2,
		IslandOpacity: 0.85,
	}
}

func (s *SVGGenerator) GenerateSector(sector *models.Sector) string {
	s.Volumes = []*models.Volume{}
	for r := 0; r < sector.Height; r++ {
		for c := 0; c < sector.Width; c++ {
			if vol := sector.Volumes[r][c]; vol != nil && !vol.IsEmpty() {
				s.Volumes = append(s.Volumes, vol)
			}
		}
	}

	var svg strings.Builder
	svg.WriteString(s.header())
	svg.WriteString(s.tractMarks())
	svg.WriteString(s.hexGrid())
	svg.WriteString(s.islands())
	svg.WriteString(s.buildRoutes())
	for _, v := range s.Volumes {
		svg.WriteString(s.world(v))
	}
	svg.WriteString(s.volumeNumbers())
	svg.WriteString(s.frame())
	svg.WriteString(s.footer())

	return svg.String()
}

func (s *SVGGenerator) centerOf(col, row int) (float64, float64) {
	x := s.Side + float64(col-1)*s.Side*1.5
	y := float64(row-1)*s.Side*s.Factor + (s.Side * s.Factor / (1 + float64(col%2)))
	return x, y
}

func (s *SVGGenerator) world(volume *models.Volume) string {
	if volume.Star == nil || volume.Star.World == nil {
		return ""
	}

	w := volume.Star.World
	col := volume.Column
	row := volume.Row
	cx, cy := s.centerOf(col, row)
	curve := s.Side / 2

	var output strings.Builder
	output.WriteString(fmt.Sprintf("<!-- Volume: %s -->\n", volume.Location()))

	if w.Size == 0 {
		output.WriteString(s.drawBelt(cx, cy))
	} else {
		output.WriteString(s.drawPlanet(cx, cy, w))
	}

	output.WriteString(fmt.Sprintf("    <text class='spaceport' x='%d' y='%d'>%s</text>\n",
		int(cx), int(cy+s.Side/2), w.Port))

	output.WriteString(fmt.Sprintf("    <text x='%d' y='%d'>%s</text>\n",
		int(cx), int(cy+s.Side/1.3), w.GetUWP()))

	output.WriteString(fmt.Sprintf("    <text x='%d' y='%d'>%s</text>\n",
		int(cx), int(cy-s.Side/2.1), volume.Name))

	if w.TravelCode != "." {
		output.WriteString(fmt.Sprintf("    <path class='zone' d='M %d %d a %.0f %.0f 0 1 0 20 0' />\n",
			int(cx-curve/2), int(cy-curve/1.4), curve, curve))
	}

	if strings.Contains(w.Bases, "N") {
		output.WriteString(s.navyBase(cx, cy))
	}
	if strings.Contains(w.Bases, "S") {
		output.WriteString(s.scoutBase(cx, cy))
	}
	if volume.GasGiant == "G" {
		output.WriteString(s.gasGiant(cx, cy))
	}

	output.WriteString(s.stars(cx, cy, volume.Star))

	return output.String()
}

func (s *SVGGenerator) drawPlanet(cx, cy float64, w *models.World) string {
	return fmt.Sprintf("    <circle class='planet' cx='%d' cy='%d' r='%d' />\n",
		int(cx), int(cy), int(s.Side/7))
}

func (s *SVGGenerator) drawBelt(cx, cy float64) string {
	var output strings.Builder
	output.WriteString("    <g class='belt'>\n")
	for i := 0; i < 7; i++ {
		x := cx + (math.Floor(math.Mod(float64(i*17), s.Side/3)) - s.Side/6)
		y := cy + (math.Floor(math.Mod(float64(i*23), s.Side/3)) - s.Side/6)
		output.WriteString(fmt.Sprintf("      <circle class='belt' cx='%d' cy='%d' r='%.1f' />\n",
			int(x), int(y), s.Side/15))
	}
	output.WriteString("    </g>\n")
	return output.String()
}

func (s *SVGGenerator) gasGiant(cx, cy float64) string {
	x := cx + s.Side/1.8
	y := cy + s.Side/3
	return fmt.Sprintf(`    <g class='gas-giant'><!-- Has Gas Giant -->
      <ellipse cx='%d' cy='%d' rx='%d' ry='%.1f' />
      <circle  cx='%d' cy='%d' r='%d' />
    </g>
`, int(x), int(y), int(s.Side/(s.Mark*0.5)), s.Side/s.Mark*0.3,
		int(x), int(y), int(s.Side/(s.Mark*1.2)))
}

func (s *SVGGenerator) symbol(name, symbol string, x, y float64) string {
	return fmt.Sprintf("    <!-- %s --><text class='symbol %s' x='%d' y='%d'>%s</text>\n",
		name, name, int(x), int(y), symbol)
}

func (s *SVGGenerator) scoutBase(cx, cy float64) string {
	return s.symbol("S", "⚜", cx-s.Side/1.8, cy+s.Side/2.4)
}

func (s *SVGGenerator) navyBase(cx, cy float64) string {
	return s.symbol("N", "⚓", cx-s.Side/1.8, cy-s.Side/6)
}

func (s *SVGGenerator) stars(cx, cy float64, star *models.Star) string {
	var output strings.Builder
	x := cx + s.Side/1.8 + 2
	y := cy - s.Side/3 + 3

	output.WriteString(fmt.Sprintf("    <text x='%d' y='%d'>%s</text>\n",
		int(x), int(y), star.Classification()[:2]))

	for _, comp := range star.Companions {
		x += 3
		y += 7
		output.WriteString(fmt.Sprintf("    <text x='%d' y='%d'>%s</text>\n",
			int(x), int(y), comp.Classification()[:2]))
	}

	return output.String()
}

// islands outlines clusters of nearby systems, reusing the shared Borders geometry.
func (s *SVGGenerator) islands() string {
	if !s.ShowIslands {
		return ""
	}
	hexes := make([]hexCell, 0, len(s.Volumes))
	for _, v := range s.Volumes {
		hexes = append(hexes, hexCell{v.Column, v.Row})
	}
	groups := Borders(hexes, s.Side, s.Factor, s.Columns, s.Rows, s.IslandJump, s.IslandMin)
	if len(groups) == 0 {
		return ""
	}
	var out strings.Builder
	out.WriteString("<g class='islands'>\n")
	for _, g := range groups {
		out.WriteString(fmt.Sprintf("  <g><!--island n=%d-->", g.Size))
		for _, ring := range g.Loops {
			pts := make([]string, len(ring))
			for i, p := range ring {
				pts[i] = fmt.Sprintf("%d,%d", p.X, p.Y)
			}
			out.WriteString(fmt.Sprintf("<polygon points='%s' stroke='%s' style='opacity:%g'/>",
				strings.Join(pts, " "), g.Colour, s.IslandOpacity))
		}
		out.WriteString("</g>\n")
	}
	out.WriteString("</g><!--/islands-->\n")
	return out.String()
}

// buildRoutes draws jump routes between systems, a port of Ruby's build_routes /
// calc_route: per source it considers nearby targets and keeps one line per slope
// bucket (and never the reverse of an existing route), avoiding overlap clutter.
func (s *SVGGenerator) buildRoutes() string {
	s.Routes = map[string][]string{}
	s.Slopes = map[string][]float64{}

	present := map[[2]int]bool{}
	for _, v := range s.Volumes {
		present[[2]int{v.Column, v.Row}] = true
	}
	keys := make([][2]int, 0, len(s.Volumes))
	for _, v := range s.Volumes {
		keys = append(keys, [2]int{v.Column, v.Row})
	}
	sort.Slice(keys, func(i, j int) bool { return hexStr(keys[i]) < hexStr(keys[j]) })

	var out strings.Builder
	out.WriteString("<g class='routes'>\n")
	for _, src := range keys {
		for _, tgt := range eachHexTargets(src) {
			if !present[tgt] {
				continue
			}
			if line := s.calcRoute(src, tgt); line != "" {
				out.WriteString(line + "\n")
			}
		}
	}
	out.WriteString("</g>\n")
	return out.String()
}

// eachHexTargets yields the candidate targets around a source, in Ruby each_hex order.
func eachHexTargets(src [2]int) [][2]int {
	x, y := src[0], src[1]
	ranges := [5][2]int{{-4, 4}, {-4, 3}, {-3, 3}, {-3, 2}, {-2, 2}}
	var out [][2]int
	for index := 0; index < 5; index++ {
		lo, hi := ranges[index][0], ranges[index][1]
		for _, i := range []int{index, -index} {
			x1 := x + i
			for j := lo; j <= hi; j++ {
				out = append(out, [2]int{x1, y + j})
			}
		}
	}
	return out
}

func (s *SVGGenerator) calcRoute(src, tgt [2]int) string {
	shex, thex := hexStr(src), hexStr(tgt)
	sx, sy := s.centerOf(src[0], src[1])
	dx, dy := s.centerOf(tgt[0], tgt[1])
	srcPix := [2]int{int(sx), int(sy)}
	dstPix := [2]int{int(dx), int(dy)}

	d := distHex(src, tgt)
	if d == 0 || d > 4 { // self, or beyond jump-4, the widest class the stylesheet draws
		return ""
	}
	if containsStr(s.Routes[shex], thex) { // already have the reverse route
		return ""
	}
	s.Routes[thex] = append(s.Routes[thex], shex)

	m := slopePix(srcPix, dstPix)
	if containsFloat(s.Slopes[shex], m) { // already a route on this slope
		return ""
	}
	s.Slopes[shex] = append(s.Slopes[shex], m)

	return fmt.Sprintf("<!-- %s:%s --><line class='line%d' x1='%d' y1='%d' x2='%d' y2='%d' />",
		shex, thex, d, srcPix[0], srcPix[1], dstPix[0], dstPix[1])
}

func hexStr(h [2]int) string { return fmt.Sprintf("%02d%02d", h[0], h[1]) }

// distHex is the exact Traveller jump distance (hex-path count with the
// even-column stagger), the same metric the island borders use. The former
// rounded straight-line distance was off by one on staggered pairs.
func distHex(a, b [2]int) int {
	return models.HexJump(a[0], a[1], b[0], b[1])
}

// slopePix mirrors Ruby Array#slope: floored integer slope of two pixel points,
// with its 0.01 (vertical) and 0.1 (rightward-flat) special cases, rounded to 0.1.
func slopePix(a, b [2]int) float64 {
	var answer float64
	if b[0]-a[0] == 0 {
		answer = 0.01
	} else {
		answer = float64(floorDiv(b[1]-a[1], b[0]-a[0]))
	}
	if a[0] < b[0] && answer == 0 {
		answer = 0.1
	}
	return math.Round(answer*10) / 10
}

func floorDiv(a, b int) int {
	q := a / b
	if (a%b != 0) && ((a < 0) != (b < 0)) {
		q--
	}
	return q
}

func containsStr(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}

func containsFloat(list []float64, v float64) bool {
	for _, x := range list {
		if x == v {
			return true
		}
	}
	return false
}

func (s *SVGGenerator) tractMarks() string {
	var output strings.Builder
	output.WriteString("<g class='tract'>\n")

	// Subsector labels in standard Traveller order
	letters := []string{"A", "B", "C", "D", "E", "F", "G", "H", "J", "K", "L", "M", "N", "O", "P", "R"}

	// Each subsector is 10 hexes high and 8 hexes wide
	subsectorHeight := 10 * s.Side * s.Factor // 10 hexes high
	subsectorWidth := 8 * s.Side * 1.5        // 8 hexes wide (accounting for hex overlap)

	idx := 0
	for row := 0; row < 4; row++ { // 4 subsectors vertically
		y := float64(row) * subsectorHeight
		for col := 0; col < 4; col++ { // 4 subsectors horizontally
			x := float64(col) * subsectorWidth

			if idx < len(letters) {
				output.WriteString(fmt.Sprintf("<rect x='%.0f' y='%.0f' width='%.0f' height='%.0f' />",
					x, y, subsectorWidth, subsectorHeight))
				output.WriteString(fmt.Sprintf("<text x='%.0f' y='%.0f'>%s</text>\n",
					x+70, y+110, letters[idx]))
			}
			idx++
		}
	}

	output.WriteString("</g>\n")
	output.WriteString(fmt.Sprintf("<text class='namestamp' x='30' y='%d'>%s</text>\n", s.Height-40, s.Name))
	return output.String()
}

func (s *SVGGenerator) volumeNumbers() string {
	var output strings.Builder
	output.WriteString("<g class='volumes'>")

	for r := 1; r <= s.Rows+1; r++ {
		for c := 1; c <= s.Columns; c++ {
			x := s.Side + float64(c-1)*s.Side*1.5
			y := float64(r-1)*s.Side*s.Factor + s.Side*0.2
			if c%2 == 0 {
				y += s.Side * s.Factor / 2
			}
			output.WriteString(fmt.Sprintf("<text x='%d' y='%d'>%02d%02d</text>\n",
				int(x), int(y), c, r))
		}
	}

	output.WriteString("</g>")
	return output.String()
}

func (s *SVGGenerator) hexGrid() string {
	var output strings.Builder
	for j := 0; j < s.Rows*3+2; j++ {
		output.WriteString(s.hexRow(j/2, j%2 != 0))
		output.WriteString("\n")
	}
	return output.String()
}

func (s *SVGGenerator) hexRow(row int, top bool) string {
	sideH := s.Side * s.Factor / 2
	sideW := s.Side / 2
	ly := float64(row*2)*sideH + sideH

	var points []string
	for j := 0; j <= s.Columns/2; j++ {
		x := float64(j) * s.Side * 3
		y := ly
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))

		x += sideW
		if top {
			y -= sideH
		} else {
			y += sideH
		}
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))

		x += s.Side
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))

		x += sideW
		if top {
			y += sideH
		} else {
			y -= sideH
		}
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))

		x += s.Side
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))
	}

	x := float64(s.Columns/2)*s.Side*3 + s.Side*2 + sideW
	if top {
		y := ly - sideH
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))
	} else {
		y := ly + sideH
		points = append(points, fmt.Sprintf("%d,%d", int(x), int(y)))
	}

	return fmt.Sprintf("    <polyline points='%s' />", strings.Join(points, " "))
}

func (s *SVGGenerator) frame() string {
	return fmt.Sprintf("    <polyline class='frame' points='0,0 %d,0 %d,%d 0,%d 0,0' />",
		s.Width, s.Width, s.Height, s.Height)
}

func (s *SVGGenerator) footer() string {
	return "</svg>"
}

func (s *SVGGenerator) header() string {
	return fmt.Sprintf(`<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"
  "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg class='dark' width="%dpx" height="%dpx" version="1.1" xmlns="http://www.w3.org/2000/svg">
  <desc>%s Subsector Map Grid</desc>
  <style>
  text {
    text-anchor: middle;
    font: 8px sans-serif;
  }
  .tract text {
    text-anchor: left;
    font: 120px sans-serif;
  }
  text.namestamp {
    text-anchor: start;
    font-size: 36px;
  }
  text.symbol {
    font-size: 14px;
  }
  text.symbol.N {
    font-size: 9px;
  }
  g.volumes text {
    opacity: 0.5;
  }
  line {
    opacity: 0.3;
    stroke-linecap: round;
  }
  line.line1 {
    stroke-width:4;
  }
  line.line2 {
    stroke-width:3;
  }
  line.line3 {
    stroke-width:2;
    stroke-dasharray: 5, 5, 1, 5;
    opacity: 0.6;
  }
  line.line4 {
    stroke-width:1.5;
    stroke-dasharray: 2,6;
  }
  polyline {
    fill: none;
    stroke-width: 1;
  }
  g.gas-giant circle {
    stroke-width: 2;
  }
  g.gas-giant ellipse {
    stroke-width: 1;
  }
  circle {
    stroke-width: 1;
  }
  .zone {
    fill: none;
    stroke-width: 3;
    stroke-dasharray: 3,6;
    stroke-linecap: round;
  }
  g.islands polygon {
    stroke-width: 6;
    fill: none;
    stroke-linejoin: round;
  }
  @media (prefers-color-scheme: light) {
      svg {
        fill: #FAFAFA;
      }
       text {
        fill: #383A42;
      }
       .tract text {
        fill: #ABB2BF;
      }
       text.symbol {
        fill: #121417;
      }
       g.volumes text {
        fill: #383A42;
      }
       line.line1 {
        stroke: #50A14F;
      }
       line.line2 {
        stroke: #4078F2;
      }
       line.line3 {
        stroke: #986801;
      }
       line.line4 {
         opacity: 0.6;
        stroke: #A626A4;
      }
      rect, polyline {
        stroke: #ABB2BF;
      }
       g.gas-giant circle {
        stroke: #383A42;
        fill:   #383A42;
      }
       g.gas-giant ellipse {
        stroke: #383A42;
      }
       circle {
        fill: #121417;
        stroke: #FAFAFA;
      }
       .zone {
        stroke: #B90;
      }
    }
    @media (prefers-color-scheme: dark) {
      svg {
        fill: #121417;
      }
      text {
        fill: #ABB2BF;
      }
      .tract text {
        fill: #FFF;
        opacity: 0.1;
      }
      text.symbol {
        fill: #ABB2BF;
      }
      g.volumes text {
        fill: #ABB2BF;
      }
      line.line1 {
        stroke: #9F9;
      }
      line.line2 {
        stroke: #F99;
        opacity: 0.5;
      }
      line.line3 {
        stroke: #F90;
      }
      line.line4 {
        stroke: #C6F;
        opacity: 0.6;
      }
      rect, polyline {
        stroke: #434649;
      }
      g.gas-giant circle {
        stroke: #ABB2BF;
        fill:   #ABB2BF;
      }
      g.gas-giant ellipse {
        stroke: #ABB2BF;
      }
      circle {
        fill: #999;
        stroke: none;
      }
      .zone {
        stroke: #FC3;
      }
    }
  </style>
  <rect width='%d' height='%d' />
`, s.Width, s.Height, s.Name, s.Width, s.Height)
}
