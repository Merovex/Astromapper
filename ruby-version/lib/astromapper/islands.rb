module Astromapper
  # Computes "island" cluster borders from a set of system hexes: groups nearby
  # systems, hugs each group with a tight contiguous territory, and traces the
  # perimeter along the hex grid lines.
  #
  # Pure geometry with NO gem dependencies, so it is shared by both the SVG
  # generator (Astromapper::Svg#islands) and the standalone post-processor
  # (tools/island-borders.rb). Callers pass the system hexes plus the same
  # @side/@factor the renderer uses, and get back coloured line segments to emit.
  module Islands
    PALETTE = %w[#e6194B #3cb44b #4363d8 #f58231 #911eb4 #42d4f4 #f032e6 #bfef45
                 #469990 #9A6324 #800000 #808000 #000075 #e6beff].freeze

    module_function

    # Hex centre in pixel space — identical maths to Svg#center_of so borders
    # land exactly on the grid (even columns sit half a hex lower).
    def centre(col, row, side, factor)
      x = side + (col - 1) * side * 1.5
      y = (row - 1) * side * factor + (side * factor / (1 + (col % 2)))
      [x, y]
    end

    # Traveller hex jump distance. Even columns carry the +1 offset so the metric
    # matches centre()'s column parity.
    def jump(a, b)
      ax, ay = a; bx, by = b
      ay2 = ay * 2 + (ax.even? ? 1 : 0)
      by2 = by * 2 + (bx.even? ? 1 : 0)
      dx  = (bx - ax).abs
      dy  = (by2 - ay2).abs
      (dx + [0, (dy - dx) / 2.0].max).round
    end

    # Chain a bag of unordered perimeter edges ([[a, b], ...] of points) into
    # ordered closed loops, so each island can be drawn as a single polygon
    # (clean per-element opacity, no overlapping segments at the vertices).
    def chain_loops(edges)
      adj = Hash.new { |h, k| h[k] = [] }
      edges.each { |a, b| adj[a] << b; adj[b] << a }
      used = {}
      loops = []
      edges.each do |a, b|
        ek = [a, b].sort
        next if used[ek]
        used[ek] = true
        ring = [a]
        prev = a; cur = b
        until cur == a
          nxt = adj[cur].find { |n| n != prev && !used[[cur, n].sort] } ||
                adj[cur].find { |n| !used[[cur, n].sort] }
          break unless nxt
          used[[cur, nxt].sort] = true
          ring << cur
          prev, cur = cur, nxt
        end
        loops << ring if ring.size >= 3
      end
      loops
    end

    # hexes: array of [col, row]. Returns an array of [colour, loops, size, members]
    # where loops is a list of closed rings (each an array of [x, y] points).
    def borders(hexes, side:, factor:, cols: 32, rows: 40, threshold: 2, min_size: 2)
      hexes = hexes.uniq
      return [] if hexes.empty?

      # --- cluster the systems (union-find over jump <= threshold) ---
      parent = {}; hexes.each { |h| parent[h] = h }
      find = lambda { |x| x = parent[x] while parent[x] != x; x }
      hexes.combination(2) do |a, b|
        next unless jump(a, b) <= threshold && find.(a) != find.(b)
        parent[find.(a)] = find.(b)
      end
      clusters = hexes.group_by { |h| find.(h) }.values.select { |c| c.size >= min_size }

      half_w = side / 2.0
      half_h = side * factor / 2.0
      corners = [[side, 0], [half_w, half_h], [-half_w, half_h],
                 [-side, 0], [-half_w, -half_h], [half_w, -half_h]]

      neighbours = lambda do |c, r|
        out = []
        ((c - 1)..(c + 1)).each do |nc|
          next unless (1..cols).cover?(nc)
          ((r - 1)..(r + 1)).each do |nr|
            next unless (1..rows).cover?(nr)
            out << [nc, nr] if !(nc == c && nr == r) && jump([c, r], [nc, nr]) == 1
          end
        end
        out
      end

      # A cluster's territory: system hexes + bridge hexes (adjacent to >= 2 systems) +
      # a compactness close (absorb empty hexes with >= 3 neighbours inside, filling
      # notches and pinwheel voids). Full territory — systems are never eroded.
      build_territory = lambda do |systems|
        territory = {}
        systems.each { |s| territory[s] = true }
        adj = Hash.new(0)
        systems.each do |sc, sr|
          ((sc - 2)..(sc + 2)).each do |c|
            next unless (1..cols).cover?(c)
            ((sr - 2)..(sr + 2)).each do |r|
              next unless (1..rows).cover?(r)
              adj[[c, r]] += 1 if jump([sc, sr], [c, r]) == 1
            end
          end
        end
        adj.each { |h, n| territory[h] = true if n >= 2 }
        loop do
          candidates = {}
          territory.each_key { |c, r| neighbours.(c, r).each { |n| candidates[n] = true unless territory[n] } }
          add = candidates.keys.select { |h| neighbours.(*h).count { |n| territory[n] } >= 3 }
          break if add.empty?
          add.each { |h| territory[h] = true }
        end
        territory
      end

      # Build each island's territory, then give any hex claimed by two islands to the
      # NEAREST cluster (smallest jump to one of its systems). A system hex's nearest
      # cluster is always its own (jump 0), so systems are never moved or dropped; only
      # over-expanded close hexes get split. Removes the region-overlap while keeping
      # every island intact and every system enclosed.
      territories = clusters.map { |c| build_territory.(c) }
      owner = Hash.new { |h, k| h[k] = [] }
      territories.each_with_index { |t, i| t.each_key { |h| owner[h] << i } }
      owner.each do |h, ids|
        next if ids.size == 1
        keep = ids.min_by { |i| clusters[i].map { |s| jump(h, s) }.min }
        (ids - [keep]).each { |i| territories[i].delete(h) }
      end

      # Trace each island's perimeter. Each hex contributes 6 edges; shared (interior)
      # edges appear twice and cancel, leaving exactly the perimeter.
      territories.each_with_index.map do |territory, i|
        edges = Hash.new(0)
        raw   = {}
        territory.each_key do |c, r|
          cx, cy = centre(c, r, side, factor)
          pts = corners.map { |dx, dy| [(cx + dx).round, (cy + dy).round] }
          6.times do |k|
            a, b = pts[k], pts[(k + 1) % 6]
            key  = [a, b].sort
            edges[key] += 1
            raw[key]   = [a, b]
          end
        end
        border = edges.select { |_, n| n == 1 }.keys.map { |k| raw[k] }
        # Fourth element: the member system hexes, for callers that label islands.
        [PALETTE[i % PALETTE.size], chain_loops(border), clusters[i].size, clusters[i]]
      end
    end
  end
end
