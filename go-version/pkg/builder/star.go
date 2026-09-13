package builder

import (
	"astromapper/pkg/models"
	"astromapper/pkg/rng"
	"math"
)

var BodeRatios = []float64{0.3, 0.3, 0.3, 0.3, 0.35, 0.35, 0.35, 0.4, 0.4, 0.4, 0.4}
var CompanionSeparation = []float64{
	0.05, 0.05, 0.5, 0.5, 0.5, 2.0, 2.0, 10.0, 10.0, 10.0,
	50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0,
}

var SpectralTypes = map[models.StarType][]int{
	"O": {9},
	"B": {0, 2, 5, 8},
	"A": {0, 2, 5},
	"F": {0, 2, 5},
	"G": {0, 2, 5, 8},
	"K": {0, 2, 5},
	"M": {0, 2, 4, 6},
}

// starSequence is the O..M cooling order used for companion class derivation.
var starSequence = []models.StarType{"O", "B", "A", "F", "G", "K", "M"}

// toss mirrors Ruby toss(a,b): a d6 minus b, floored at 0.
func toss(r *rng.RNG, dice, minus int) int {
	v := r.Roll(dice, 6) - minus
	if v < 0 {
		return 0
	}
	return v
}

func BuildStar(volume *models.Volume, primary *models.Star, ternary int, r *rng.RNG) *models.Star {
	star := &models.Star{
		Volume:     volume,
		Primary:    primary,
		Companions: []*models.Star{},
		Orbits:     []models.Orbit{},
	}

	if primary == nil {
		star.Orbit = 0
		// Genre = realism<->romance stellar slider. opera (and half of normal) uses the
		// Sun-like T5 table; firm (and the other half of normal) uses the realistic,
		// M-dwarf-heavy census. Mirrors the Ruby Star#initialize.
		if activeGenre == "opera" || (activeGenre == "normal" && r.D6() <= 3) {
			f := r.FluxRoll() // Traveller 5 spectral table (page 436), Flux -> type
			switch {
			case f <= -4:
				star.StarType = "A"
			case f <= -2:
				star.StarType = "F"
			case f <= 0:
				star.StarType = "G"
			case f <= 2:
				star.StarType = "K"
			default:
				star.StarType = "M"
			}
			if star.StarType == "M" && r.D6() <= 3 {
				star.StarType = "K" // shift ~half the M dwarfs to K
			}
			star.TypeDM = map[models.StarType]int{"A": 2, "F": 4, "G": 6, "K": 8, "M": 10}[star.StarType]
		} else {
			natural := r.TwoD6()
			if natural >= 12 {
				// Hot stars (A/B/O) — rare (natural 12 only), ~1% as in the real galaxy.
				star.TypeDM = natural
				star.StarType = []models.StarType{"A", "A", "A", "A", "A", "A", "A", "A", "A", "A", "B", "B", "O"}[r.TwoD6()]
			} else {
				dm := natural + getStarDM(volume)
				if dm > 11 {
					dm = 11
				}
				star.TypeDM = dm
				star.StarType = []models.StarType{"M", "M", "F", "M", "M", "M", "M", "M", "K", "K", "G", "M"}[dm]
			}
		}

		star.SizeDM = min(r.TwoD6(), 12)
		starSizes := []int{0, 1, 2, 3, 4, 5, 5, 5, 5, 5, 5, 6, 500}
		star.StarSize = starSizes[star.SizeDM]
	} else {
		// Separation: 2d6 x GURPS multiplier; index is toss(3) [3d6-2] + 4*ternary - 2.
		// Ruby's index -1 wraps to the last entry (50 AU) — preserved.
		separationIndex := (r.ThreeD6() - 2) + (4 * ternary) - 2
		if separationIndex < 0 {
			separationIndex += len(CompanionSeparation)
		}
		if separationIndex >= len(CompanionSeparation) {
			separationIndex = len(CompanionSeparation) - 1
		}
		separation := float64(r.TwoD6()) * CompanionSeparation[separationIndex]
		separation = math.Round(separation*100) / 100

		star.Orbit = star.AUToOrbit(separation) - 1

		// Companion class feeds off the primary: same class or cooler by 1D-1 steps
		// down the O B A F G K M sequence (T5 page 436; Ruby Star#initialize).
		pidx := len(starSequence) - 1
		for i, t := range starSequence {
			if t == primary.StarType {
				pidx = i
				break
			}
		}
		cidx := pidx + (r.D6() - 1)
		if cidx > len(starSequence)-1 {
			cidx = len(starSequence) - 1
		}
		star.StarType = starSequence[cidx]

		companionSizes := []int{0, 1, 2, 3, 4, 500, 500, 5, 5, 6, 500, 500, 500, 500}
		star.StarSize = companionSizes[min(r.TwoD6()+primary.SizeDM, 12)]
	}

	spectralSubtypes := SpectralTypes[star.StarType]
	subtype := spectralSubtypes[r.Intn(len(spectralSubtypes))]
	star.Spectral = string(star.StarType) + string(rune('0'+subtype))

	if star.StarSize == 500 {
		star.StarSubtype = "B"
		star.StarType = "D"
	}

	if star.StarType == "M" && star.StarSize == 5 {
		star.BodeConstant = 0.2
	} else {
		star.BodeConstant = BodeRatios[toss(r, 2, 2)]
	}

	// Companions don't grow their own displayed orbit systems (Ruby generates and
	// discards them; we skip the wasted draws — per-language RNG parity only).
	if primary != nil {
		return star
	}

	dm := 0
	if star.StarSize == 3 {
		dm += 4
	}
	if star.StarSize < 3 {
		dm += 8
	}
	if star.StarType == "M" {
		dm -= 4
	}
	if star.StarType == "K" {
		dm -= 2
	}

	numOrbits := max(r.TwoD6()+dm, 0)
	for i := 0; i < numOrbits; i++ {
		au := star.OrbitToAU(i)
		if au > star.OuterLimit() {
			break
		}

		orbit := populateOrbit(star, i, r)
		star.Orbits = append(star.Orbits, orbit)

		if world, ok := orbit.(*models.World); ok {
			star.World = world
		}
	}

	if star.World != nil {
		gasGiants, belts := 0, 0
		for _, orbit := range star.Orbits {
			switch orbit.GetKid() {
			case models.OrbitGasGiant:
				gasGiants++
			case models.OrbitBelt:
				belts++
			}
		}
		if gasGiants > 0 {
			star.World.GasGiant = "G"
		} else {
			star.World.GasGiant = "."
		}
		// Extensions (Ix/Ex/Cx + RU) need the whole-system gas-giant/belt counts.
		buildExtensions(star.World, gasGiants, belts, r)
	}

	pruneOrbits(star)

	return star
}

func populateOrbit(star *models.Star, orbitNum int, r *rng.RNG) models.Orbit {
	au := star.OrbitToAU(orbitNum)

	if au < star.InnerLimit() {
		return &models.EmptyOrbit{
			BaseOrbit: models.BaseOrbit{
				Star:        star,
				OrbitNumber: orbitNum,
				AU:          au,
				Kid:         models.OrbitEmpty,
			},
		}
	}

	// No biozone entry: every orbit reads as inner (Ruby's rescue branch).
	biozone, hasBiozone := star.GetBiozone()
	zone := -1
	distant := true
	if hasBiozone {
		zone = 0
		if au < biozone[0] {
			zone = -1
		} else if au > biozone[1] {
			zone = 1
		}
		distant = au > biozone[1]*10
	}

	if zone == 0 {
		return populateBiozone(star, orbitNum, r)
	} else if zone < 0 {
		return populateInner(star, orbitNum, au, zone, distant, r)
	}
	return populateOuter(star, orbitNum, au, zone, distant, r)
}

// populateBiozone: with always_inhabited (the default) the biozone orbit is a
// guaranteed mainworld; otherwise 2d6 < 12 world, 12 gas giant (Ruby).
func populateBiozone(star *models.Star, orbitNum int, r *rng.RNG) models.Orbit {
	if activeAlwaysInhabited || r.TwoD6() < 12 {
		return BuildWorld(star, orbitNum, r)
	}
	au := star.OrbitToAU(orbitNum)
	return makeGasGiant(star, orbitNum, au, 0, false, r)
}

// populateInner: 2d6 — <5 empty, 5-6 hostile, 7-9 rockball, 10-11 belt, 12 gas giant.
func populateInner(star *models.Star, orbitNum int, au float64, zone int, distant bool, r *rng.RNG) models.Orbit {
	roll := r.TwoD6()

	base := models.BaseOrbit{
		Star:        star,
		OrbitNumber: orbitNum,
		AU:          au,
		Zone:        zone,
		Distant:     distant,
		Port:        "X",
	}

	switch {
	case roll < 5:
		base.Kid = models.OrbitEmpty
		base.Port = ""
		return &models.EmptyOrbit{BaseOrbit: base}
	case roll <= 6:
		return makeHostile(base, r)
	case roll <= 9:
		return makeRockball(base, r)
	case roll <= 11:
		base.Kid = models.OrbitBelt
		return &models.Belt{BaseOrbit: base}
	default:
		return makeGasGiant(star, orbitNum, au, zone, distant, r)
	}
}

// populateOuter: 1d6 (+1 if beyond 10x the biozone) — 1 rockball, 2 belt, 3 empty,
// 4-7 gas giant.
func populateOuter(star *models.Star, orbitNum int, au float64, zone int, distant bool, r *rng.RNG) models.Orbit {
	roll := r.D6()
	if distant {
		roll++
	}

	base := models.BaseOrbit{
		Star:        star,
		OrbitNumber: orbitNum,
		AU:          au,
		Zone:        zone,
		Distant:     distant,
		Port:        "X",
	}

	switch {
	case roll == 1:
		return makeRockball(base, r)
	case roll == 2:
		base.Kid = models.OrbitBelt
		return &models.Belt{BaseOrbit: base}
	case roll == 3:
		base.Kid = models.OrbitEmpty
		base.Port = ""
		return &models.EmptyOrbit{BaseOrbit: base}
	case roll <= 7:
		return makeGasGiant(star, orbitNum, au, zone, distant, r)
	default:
		return makeRockball(base, r)
	}
}

// makeRockball: airless world — size 2d6-2, up to 1d6-3 moons (Ruby Planet defaults).
func makeRockball(base models.BaseOrbit, r *rng.RNG) models.Orbit {
	base.Kid = models.OrbitRockball
	base.Size = toss(r, 2, 2)
	base.Moons = generateMoons(toss(r, 1, 3), base.Size, "", base.Zone, r)
	return &models.Rockball{BaseOrbit: base}
}

// makeHostile: exotic atmosphere 10-14, (acid) seas 2d6-4 capped at A; moons as a
// normal planet.
func makeHostile(base models.BaseOrbit, r *rng.RNG) models.Orbit {
	base.Kid = models.OrbitHostile
	base.Size = toss(r, 2, 2)
	base.Moons = generateMoons(toss(r, 1, 3), base.Size, "", base.Zone, r)
	base.Atmosphere = []int{10, 11, 12, 13, 14}[r.Intn(5)]
	base.Hydro = min(toss(r, 2, 4), 10)
	return &models.Hostile{BaseOrbit: base}
}

// makeGasGiant: large on 1d6 1-3; moons 2d6 (small: -4), sized by the giant.
func makeGasGiant(star *models.Star, orbitNum int, au float64, zone int, distant bool, r *rng.RNG) models.Orbit {
	gg := &models.GasGiant{BaseOrbit: models.BaseOrbit{
		Star:        star,
		OrbitNumber: orbitNum,
		AU:          au,
		Zone:        zone,
		Distant:     distant,
		Kid:         models.OrbitGasGiant,
	}}
	if r.D6() < 4 {
		gg.GiantSize = "L"
	} else {
		gg.GiantSize = "S"
	}
	numMoons := r.TwoD6()
	if gg.GiantSize == "S" {
		numMoons = max(numMoons-4, 0)
	}
	gg.Moons = generateMoons(numMoons, 0, gg.GiantSize, zone, r)
	return gg
}

// pruneOrbits mirrors Ruby Star#prune!: strip trailing empties, then (when at least
// two orbits remain) renumber sequentially and recompute AU.
func pruneOrbits(star *models.Star) {
	lastNonEmpty := -1
	for i := len(star.Orbits) - 1; i >= 0; i-- {
		if star.Orbits[i].GetKid() != models.OrbitEmpty {
			lastNonEmpty = i
			break
		}
	}
	star.Orbits = star.Orbits[:lastNonEmpty+1]

	if len(star.Orbits) < 2 {
		return
	}
	for i, orbit := range star.Orbits {
		orbit.SetOrbitNumber(i)
		orbit.SetAU(star.OrbitToAU(i))
	}
}

func getStarDM(volume *models.Volume) int {
	return 0
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
