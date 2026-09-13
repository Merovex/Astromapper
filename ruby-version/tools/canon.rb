#!/usr/bin/env ruby
# canon.rb — overlay the "book" / canon system names onto a Teradoma map.
#
#   ruby tools/canon.rb rename    path/to/teradoma.sector   # rename systems in place
#   ruby tools/canon.rb highlight path/to/teradoma.svg      # tint the canon hexes
#
# Run order:  rename  ->  regenerate the SVG (`astromapper svg`)  ->  highlight.
# The rename edits the .sector file in place; a later `astromapper build`
# regenerates from the seed and would overwrite it, so re-run rename after a build.
#
# Highlighting reuses the shared hex geometry in lib/astromapper/islands.rb so the
# tinted hexes line up exactly with the grid.

require_relative '../lib/astromapper/islands'

# Hex -> canonical "book" name. Notes: Phrandzoi (civilian) and Sipfarejio (Navy)
# are one system at 1838 (kept as Sipfarejio); Old Sol is off-map; Shafistio and
# Kampulio are bodies inside Ellysio (1921) — none of those get their own hex.
MAP = {
  "2429" => "Adiantio",    "2119" => "Copa",       "2113" => "Corvallisio", "1834" => "Difektĝintio",
  "2013" => "Dolchio",     "0816" => "Dunatrio",   "1921" => "Ellysio",     "3028" => "Euakovio",
  "1207" => "Eudorio",     "1740" => "Guna",       "0126" => "Hadriu",      "0935" => "Johor",
  "1919" => "Katalio",     "2208" => "Keicha",     "2505" => "Korundio",    "1819" => "Krinan",
  "2316" => "Manjamio",    "2829" => "Merovingio", "2424" => "Mollan",      "0913" => "Monio",
  "2516" => "Moyaba",      "2209" => "Naqsa",      "3016" => "Ochven",      "2405" => "Pellinio",
  "2525" => "Pendrakeentio","3223" => "Pivetio",   "1838" => "Sipfarejio",  "2012" => "Quinnio",
  "1622" => "Rosendanio",  "1711" => "Sabanio",    "2010" => "Shexio",      "2919" => "Sigurd",
  "2233" => "Smyrno",      "2912" => "Sorchakvio", "2628" => "Sovaĝio",     "2522" => "Tanhusio",
  "2826" => "Temasek",     "2533" => "Tesheuquio", "1640" => "Trogio",      "2626" => "Virshafio",
  "2104" => "Xaryio",      "2728" => "Zahnah",
}.freeze

SIDE         = 40.0
FACTOR       = 1.732
FILL         = "#ffcf3f"   # canon-hex tint
FILL_OPACITY = 0.35

mode, path, list_path, fill_arg, opacity_arg = ARGV
abort "usage: ruby tools/canon.rb [rename|highlight] <file> [hexlist.txt [fill [opacity]]]" unless mode && path
# highlight may take an external hex list ("HEX name" per line, # comments) plus a
# fill and opacity, instead of the built-in canon MAP.
HIGHLIGHT = if list_path
  File.readlines(list_path, encoding: "utf-8").map(&:strip).reject { |l| l.empty? || l.start_with?("#") }
      .to_h { |l| h, n = l.split(/\s+/, 2); [h, n.to_s] }
else
  MAP
end
HL_FILL    = fill_arg    || FILL
HL_OPACITY = opacity_arg || FILL_OPACITY

case mode
when "rename"
  out = File.readlines(path, encoding: "utf-8").map do |line|
    hex = line[0, 4]
    next line unless line =~ /^\d{4}/ && MAP[hex]
    f = line.chomp.split("\t")
    f[4] = MAP[hex].ljust(15)        # field [4] is the name (crib+orbits share [3])
    f.join("\t") + "\n"
  end
  File.write(path, out.join)
  puts "renamed #{MAP.size} systems in #{path}"

when "highlight"
  svg = File.read(path, encoding: "utf-8")
  half_w = SIDE / 2; half_h = SIDE * FACTOR / 2
  corners = [[SIDE, 0], [half_w, half_h], [-half_w, half_h],
             [-SIDE, 0], [-half_w, -half_h], [half_w, -half_h]]
  polys = HIGHLIGHT.keys.sort.map do |hex|
    cx, cy = Astromapper::Islands.centre(hex[0, 2].to_i, hex[2, 2].to_i, SIDE, FACTOR)
    pts = corners.map { |dx, dy| "#{(cx + dx).round},#{(cy + dy).round}" }.join(' ')
    "<polygon points='#{pts}'><!--#{hex} #{HIGHLIGHT[hex]}--></polygon>"
  end.join("\n")
  group = "\n<g class='canon' fill='#{HL_FILL}' fill-opacity='#{HL_OPACITY}' stroke='none'>\n#{polys}\n</g><!--/canon-->"
  svg = svg.sub(%r{\n?<g class='canon'[^>]*>.*?</g><!--/canon-->}m, '')      # idempotent
  # Sit above the grid/tract rects (which carry an opaque themed fill) but below
  # the islands, routes and systems, so the tint shows yet labels stay on top.
  anchor = svg.include?("<g class='islands'>") ? "<g class='islands'>" : "<g class='routes'>"
  svg = svg.sub(anchor, "#{group}\n#{anchor}")
  File.write(path, svg)
  puts "highlighted #{HIGHLIGHT.size} hexes in #{path}"

when "tab"
  lines = File.readlines(path, encoding: "utf-8")
  hdr   = lines.index { |l| l =~ /^Sector\t|^Hex\t/ } || lines.index { |l| !l.start_with?("#") }
  cols  = lines[hdr].chomp.split("\t").each_with_index.to_h
  hi, ni = cols["Hex"], cols["Name"]
  n = 0
  out = lines.each_with_index.map do |line, i|
    next line if i <= hdr || line.start_with?("#")
    f = line.chomp.split("\t")
    if MAP[f[hi]]
      f[ni] = MAP[f[hi]]; n += 1
      f.join("\t") + "\n"
    else
      line
    end
  end
  File.write(path, out.join)
  puts "renamed #{n} systems in #{path}"

else
  abort "unknown mode #{mode.inspect}; use rename, tab or highlight"
end
