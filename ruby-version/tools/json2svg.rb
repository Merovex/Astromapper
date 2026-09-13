#!/usr/bin/env ruby
# json2svg.rb — render an SVG map from a (Go-format) Converted-Sector JSON, reusing
# the production Astromapper::Svg renderer (hex grid, worlds, starports, stars,
# bases, gas giants, jump routes, and the woven-in island borders).
#
#   ruby tools/json2svg.rb input.json [output.svg]
#
# The renderer parses the Ruby tab-format sector, so this converts each JSON volume
# into that format (only the mainworld line is rendered — orbits/moons aren't drawn),
# writes it to a temp .sector file, and runs Svg#convert on it.

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require 'json'
require 'tempfile'
require 'astromapper'

ROMAN = ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"].freeze
EHEX  = "0123456789ABCDEFGHJKLMNPQRSTUVWXYZ".freeze

def classification(s)
  return "D#{s['star_subtype']}" if s["star_type"] == "D"
  "#{s['spectral']}#{ROMAN[s['star_size']] || ''}"
end

# The JSON world carries digits, not a UWP string — assemble it.
def uwp(w)
  h = ->(n) { (n = n.to_i) < 0 ? "0" : (EHEX[n] || EHEX[-1]) }
  "%s%s%s%s%s%s%s-%s" % [w["starport"], h.(w["size"]), h.(w["atmosphere"]), h.(w["hydrographics"]),
                         h.(w["population"]), h.(w["government"]), h.(w["law_level"]), h.(w["tech_level"])]
end

# Bases/feature string the renderer scans with .include?('N'/'S'/'G'/'D'/'W').
def nsg(world, has_gg)
  b = world["bases"].to_s
  s = +""
  s << "N" if b.include?("N")
  s << "S" if b.include?("S")
  s << "G" if has_gg || b.include?("G")
  s << "D" if b.include?("D")
  s << "W" if b.include?("W")
  s.empty? ? "." : s
end

input  = ARGV[0] or abort "usage: ruby tools/json2svg.rb input.json [output.svg]"
output = ARGV[1] || input.sub(/\.json$/i, '') + ".svg"
doc = JSON.parse(File.read(input, encoding: "utf-8"))

# Config injection: the renderer reads only name + island settings from config.
module Astromapper
  class << self
    attr_writer :svg_config
    def config(_ = nil); (@svg_config || {}).with_indifferent_access; end
  end
end
Astromapper.svg_config = { "name" => doc["name"], "islands" => true,
                           "island_jump" => 2, "island_min" => 2, "island_opacity" => 0.85 }
srand(0)   # stable belt jitter

# Ruby tab-format sector line: "LOC UWP TEMP NSG TZ \t TRADE \t FACTIONS \t STARS \t NAME"
lines = doc["volumes"].values.sort_by { |v| [v["row"], v["column"]] }.map do |v|
  s = v["star"]
  # Lean JSON (no star yet, e.g. a converted map before world generation): render a
  # placeholder so the hex, name, routes and islands still draw.
  if s.nil? || s["world"].nil?
    loc = "%02d%02d" % [v["column"], v["row"]]
    next "%s\t\t\t\t%s" % ["#{loc} X000000-0 .. . ..", v["name"]]
  end
  w = s["world"]
  comp = (s["orbits"] || []).select { |o| o["type"] == "companion" }.map { |o| o["data"]["star_classification"] }.compact
  stars = ([classification(s)] + comp).join("/")
  loc = "%02d%02d" % [v["column"], v["row"]]
  details = "%s %s %s %s %s" % [loc, uwp(w), w["temperature"], nsg(w, s["has_gas_giant"]), (w["travel_code"] || "..")]
  "%s\t%s\t%s\t%s\t%s" % [details, (w["trade_codes"] || []).join(","), (w["factions"] || []).join(","), stars, v["name"]]
end

tmp = Tempfile.new(["sector", ".sector"])
tmp.write("# rendered from #{File.basename(input)}\n" + lines.join("\n") + "\n")
tmp.close

svg = Astromapper::Svg.new(tmp.path)
svg.instance_variable_set(:@svg_filename, output)
svg.convert
tmp.unlink
puts "wrote #{output}  (#{doc['volumes'].size} systems)"
