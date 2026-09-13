module Astromapper
  class Svg
    def config
      Astromapper.config
    end
    def initialize(filename)
      @rows     = 40
      @columns  = 32
      @source_filename = filename
      sectorname = filename.gsub(File.extname(filename), '')
      @svg_filename    = sectorname + '.svg'
      @side    = 40
      @factor  = 1.732
      @height  = (@side * @factor * (@rows + 0.5)).ceil
      @width   = (@side * (@columns * 1.5 + 0.5)).ceil
      @mark    = 13
      @zones   = []
      @volumes = []
      @routes  = Hash.new
      @lines   = Hash.new
      @slopes  = Hash.new
      @lines.default  = []
      @name    = config['name']
      @hex = {
        :side_h => (@side * (@factor / 2)).tweak,
        :side_w => (@side / 2).tweak,
        :width  => @side
      }
    end
    def from_file
      File.open(@source_filename,'r').readlines.each { |line| @volumes << line if /^\d{4}/.match(line) }
    end
    def calc_route(keys, source, target)
      shex = source.to_hex
      thex = target.to_hex
      [shex,thex].each { |k| @routes[k] = [] if @routes[k].nil? }
      [shex,thex].each { |k| @slopes[k] = [] if @slopes[k].nil? }

      src = center_of(source).map(&:to_i)
      dst = center_of(target).map(&:to_i)
      m = src.slope(dst)
      # Jump distance is the exact hex-path count (with the even-column stagger),
      # the same metric the island borders use; the old rounded straight line was
      # off by one on staggered pairs.
      d = Astromapper::Islands.jump(source, target)

      # Return meaningless route to self; and cap at jump-4, the widest class the
      # stylesheet draws (the scan window reaches a few jump-5 pairs on the stagger).
      return nil if d == 0 || d > 4

      # Avoid duplicate route from Target to Source
      return nil if @routes[shex].include?(thex)
      @routes[thex] << shex

      # Avoid overlapping routes.
      return nil if @slopes[shex].include?(m)
      @slopes[shex] << m

      return "<!-- #{shex}:#{thex} --><line class='line#{d}' x1='#{src[0]}' y1='#{src[1]}' x2='#{dst[0]}' y2='#{dst[1]}' />"
    end
    # Trace borders around "islands" — clusters of nearby systems. On by default;
    # disable with `islands: false` in _astromapper.yml. Reuses the shared
    # Astromapper::Islands geometry so it stays in sync with the standalone tool.
    def islands
      return '' if [false, 'false'].include?(config['islands'])
      hexes = @volumes.map { |v| [v[0..1].to_i, v[2..3].to_i] }
      groups = Astromapper::Islands.borders(
        hexes, side: @side, factor: @factor, cols: @columns, rows: @rows,
        threshold: (config['island_jump'] || 2).to_i,
        min_size:  (config['island_min']  || 2).to_i)
      return '' if groups.empty?

      opacity = config['island_opacity'] || 0.85
      out = ["<g class='islands'>"]
      groups.each do |colour, loops, size|
        polys = loops.map { |ring|
          pts = ring.map { |x, y| "#{x},#{y}" }.join(' ')
          "<polygon points='#{pts}' stroke='#{colour}' style='opacity:#{opacity}'/>" }.join
        out << "<g><!--island n=#{size}-->#{polys}</g>"
      end
      out << "</g><!--/islands-->"
      out.join("\n")
    end
    def build_routes
      routes = ["<g class='routes'>"]
      keys = @volumes.map { |v| v[0..3] }.sort
      keys.each_hex { |coordinates|
        routes << calc_route(keys, *coordinates) if keys.include?(coordinates.last.to_hex)
      }
      routes << "\n</g>"
      return routes.compact.uniq.join("\n")
    end
    def convert
      from_file

      svg = []
      svg << header
      svg << tract_marks
      svg << hex_grid
      svg << islands
      svg << build_routes
      svg << @volumes.map {|v| world(v) }
      svg << volumes
      svg << frame
      svg << footer
      File.open(@svg_filename,'w').write(svg.flatten.join("\n"))
    end
    def center_of(locx)
      if locx.is_a? String
        column = locx[0..1].to_i
        row    = locx[2..3].to_i
      else
        column = locx.first
        row    = locx.last
      end
      x      = @side + ((column - 1) * @side * 1.5) # 40 + ((COL - 1) * 60 )
      y      = (row - 1) * @side * @factor + (@side * @factor / (1 + (column % 2)))
      return [x.tweak,y.tweak]
    end
    def star_coords(r1,r2,points)
      pangle = 2*@@pi/points
      sangle = @@pi/points
      oangle = @@pi/-2
      coords = []
      points.times do |j|
        coords << [r1 * Math::cos(pangle * j + oangle), r1 * Math::sin(pangle * j + oangle)]
        coords << [r2 * Math::cos((pangle * j) + sangle + oangle), r2 * Math::sin((pangle * j) + sangle + oangle)]
      end
      return coords
    end
    def world(volume)
      # TAB 0 - World Details
      # 0. Location
      # 1. UWP
      # 2. Temp
      # 3. NSG (Features)
      # 4. Travel Zone
      # TAB 1 - Trade Codes
      # TAB 2 - Factions
      # TAB 3 - Name
      # 1101 A505223-B  ..G.. »·IC,Lo,Va       »N,O,N   »·G0V  »Omivarium
      details, trades, factions, star, name = volume.split(/\t/)

      locx, uwp, temp, nsg, zone = details.split(/\s+/)

      spaceport = uwp[0]
      size      = uwp[1]
      c         = center_of(locx) # get Location's x,y Coordinates
      curve     = @side / 2

      output =  "<!-- Volume: #{volume.strip.gsub(/\t/,' // ')} -->\n"
      output +=  (size == '0') ? draw_belt(c) : draw_planet(c,uwp)
      output += "    <text class='spaceport' x='#{c[0].to_i}' y='#{(c[1] + @side / 2).to_i}'>#{spaceport.strip}</text>\n"
      output += "    <text x='#{c[0].to_i}' y='#{(c[1]+(@side/1.3)).to_i}'>#{uwp.strip}</text>\n"
      output += "    <text x='#{c[0].to_i}' y='#{(c[1]-(@side/2.1)).to_i}'>#{name.strip}</text>\n"
      unless zone == '..'
        style = zone + '_zone'
        output += "    <path class='zone' d='M #{(c[0] - curve/2).to_i} #{(c[1] - (curve/1.4)).to_i} a #{curve} #{curve} 0 1 0 20 0' />\n"
      end
      output += navy_base(c)  if nsg.include?('N')
      output += scout_base(c) if nsg.include?('S')
      output += gas_giant(c)  if nsg.include?('G')
      output += depot(c)       if nsg.include?('D')
      output += way_station(c) if nsg.include?('W')
      output += stars(c,star)
      output

    end
    def stars(c,stars)
      output = ''
      x = (c[0]+(@side/1.8)).tweak + 2
      y = (c[1]-(@side/3)).tweak + 3
      stars.split('/').each do |star|
        output += "    <text x='#{x.to_i}' y='#{y.to_i}'>#{star[0..1].strip}</text>\n"
        x += 3
        y += 7
      end
      output
    end
    def draw_planet(c,w)
      k = (w[3] == '0') ? 'Desert' : 'Planet'
       "    <circle class='planet' cx='#{c[0].to_i}' cy='#{c[1].to_i}' r='#{(@side/7).to_i}' />\n"
    end
    def draw_belt(c)
      output = "    <g class='belt'>\n"
      7.times do
        x = c[0] + Random.rand(@side/3) - @side/6
        y = c[1] + Random.rand(@side/3) - @side/6
        output += "      <circle class='belt' cx='#{x.to_i}' cy='#{y.to_i}' r='#{(@side/15).tweak}' />\n"
      end
      output + "    </g>\n"
    end
    def frame(k='Frame')
      style = k.to_sym
      z = 0; w = @width.to_i - 0; h = @height.to_i - z;
      "    <polyline class='frame' points='#{z},#{z} #{w},#{z} #{w},#{h} #{z},#{h} #{z},#{z}' />"
    end
    def tract_marks
      output = "<g class='tract'>\n"
      letters = [
        ('A'..'D').to_a, "",
        ('E'..'H').to_a, "",
        ('J'..'M').to_a, "",
        ('N'..'Q').to_a
      ].flatten
      hs = @hex[:side_h] / 4
      height = (@height/4) - hs
      # width = ((@width - @hex[:side_w])/4).ceil - ()
      width = ((@width - (@hex[:side_w] / 2)) / 4).ceil
      y = 0;
      5.times do |r|
        x = 0
        5.times do |c|
          output << "<rect x='#{x}' y='#{y}' width='#{width}' height='#{height}' />"
          output << "<text x='#{x+70}' y='#{y+110}'>#{letters.shift}</text>"
          output << "\n"
          x += width
        end
        y += height
      end
      output << "</g>\n"
      output += namestamp
      return output
    end
    def volumes
      output = "<g class='volumes'>"
      (@rows+2).times do |r|
        (@columns+1).times do |c|
          x = @side + ((c-1) * @side * 1.5)
          y = (c % 2 == 1) ? (r-1) * @side * @factor + (0.2 * @side) : (r-1) * @side * @factor + @hex[:side_h]+ (0.2 * @side)
          output += "<text x='#{x.to_i}' y='#{y.to_i}'>#{[c,r].to_hex}</text>\n"
        end
      end
      output += "</g>"
    end
    # def polygon(x, y, sx, sy, sides=4)
    #   polygon = star_coords(sx, sy, sides).map { |c| "#{(x + c[0]).to_i},#{(y.tweak+c[1]).to_i}" }
    #   "  <polygon points='#{polygon.join(' ')}' />\n"
    # end
    def gas_giant(c)
      x = (c[0]+(@side/1.8)).tweak; y = (c[1]+(@side/3)).tweak;
      return<<-GIANT
      <g class='gas-giant'><!-- Has Gas Giant -->
        <ellipse cx='#{x.to_i}' cy='#{y.to_i}' rx='#{(@side/(@mark * 0.5)).to_i}' ry='#{(@side/@mark * 0.3).tweak}' />
        <circle  cx='#{x.to_i}' cy='#{y.to_i}' r='#{(@side/(@mark * 1.2)).to_i}' />
      </g>
      GIANT
    end
    def symbol(n,s,x,y)
      return "    <!-- #{n} --><text class='symbol #{n}' x='#{x.to_i}' y='#{y.to_i}'>#{s}</text>\n"
    end
    def way_station(c);
      return symbol('W', "\u2690", c[0]-(@side/3.1), c[1]+(@side/7)  )
    end
    def depot(c);
      return symbol("D", "\u2691", c[0]-(@side/1.5), c[1]+(@side/7)  )
    end
    def scout_base(c);
      return symbol("S", "\u269C", c[0]-(@side/1.8), c[1]+(@side/2.4))
    end
    def navy_base(c);
      return symbol("N", "\u2693", c[0]-(@side/1.8), c[1]-(@side/6)  )
    end
    def hex_grid; (@rows * 3 + 2).times.map { |j| hex_row((j/2).floor, (j % 2 != 0)) };
    end
    def namestamp
      return "<text class='namestamp' x='30' y='2800'>#{@name}</text>"
    end
    def hex_row(row, top=false)
      ly = (row * 2 * @hex[:side_h]) + @hex[:side_h]
      points = []
      x = 0; y = 0
      (@columns/2).ceil.times do |j|
        x = j * @side * 3
        y = ly
        points << "#{x.to_i},#{y.to_i}"

        x += @hex[:side_w]
        y = (top) ? y - @hex[:side_h] : y + @hex[:side_h]
        points << "#{x.to_i},#{y.to_i}"

        x += @hex[:width]
        points << "#{x.to_i},#{y.to_i}"

        x += @hex[:side_w]
        y = (top) ? y + @hex[:side_h] : y - @hex[:side_h]
        points << "#{x.to_i},#{y.to_i}"

        x += @hex[:width]
        points << "#{x.to_i},#{y.to_i}"
      end
      x += @hex[:side_w]
      y = (top) ? y - @hex[:side_h] : y + @hex[:side_h]
      points << "#{x.to_i},#{y.to_i}"
      "    <polyline points='#{points.join(' ')}' />"
    end
    def footer
      return "</svg>"
    end
    def header
      return<<-EOS
<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"
  "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg class='dark' width="#{@width}px" height="#{@height}px" version="1.1" xmlns="http://www.w3.org/2000/svg">
  <desc>#{@name} Subsector Map Grid</desc>
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
    stroke-linejoin: round;
    fill: none;
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
    @media (print) {
        svg {
          fill: #FFF;
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
          stroke: #98C379;
        }
         line.line2 {
          stroke: #61AFEF;
        }
         line.line3 {
          stroke: #F90;
        }
         line.line4 {
          stroke: #E06C75;
        }
         rect, polyline {
          stroke: #ABB2BF;
        }
         g.gas-giant circle {
          stroke: #333;
          fill:   #333;
        }
         g.gas-giant ellipse {
          stroke: #333;
        }
         circle {
          fill: #121417;
          stroke: #fff;
        }
         .zone {
          stroke: #B90;
        }

      }
    <!-- DARK THEME -->
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
  <rect width='#{@width}' height='#{@height}' />
      EOS
    end
  end
end
