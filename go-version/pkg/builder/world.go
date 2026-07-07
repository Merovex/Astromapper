package builder

import (
	"astromapper/pkg/models"
	"astromapper/pkg/rng"
)

// BuildWorld generates a mainworld via the active ruleset (Traveller 5 by default).
// The UWP step formulas, trade codes, starport/tech/base tables, and the
// climate/native modules all come from rules/<name>.yml; only the orbital framing
// (zone, AU, moons) is intrinsic to the system. Extensions (Ix/Ex/Cx) are filled in
// later by buildExtensions, once the system's gas-giant and belt counts are known.
func BuildWorld(star *models.Star, orbitNum int, r *rng.RNG) *models.World {
	rs := ruleset()

	world := &models.World{
		BaseOrbit: models.BaseOrbit{
			Star:        star,
			OrbitNumber: orbitNum,
			AU:          star.OrbitToAU(orbitNum),
			Kid:         models.OrbitWorld,
			Port:        "X",
		},
	}

	biozone, hasBiozone := star.GetBiozone()
	if hasBiozone {
		world.Zone = determineZone(world.AU, biozone)
		world.Distant = world.AU > biozone[1]*10
	} else {
		world.Zone = -1
		world.Distant = true
	}

	// UWP spine — Size / Atmo / Hydro from the ruleset's step formulas.
	ctx := map[string]any{}
	world.Size, _ = rs.UWPStep("size", ctx, r)
	ctx["size"] = world.Size
	world.Atmosphere, _ = rs.UWPStep("atmo", ctx, r)
	ctx["atmo"] = world.Atmosphere
	world.Hydro, _ = rs.UWPStep("hydro", ctx, r)
	ctx["hydro"] = world.Hydro

	world.Temperature = climate(world, r)
	ctx["temp"] = world.Temperature

	// Genre realism pass may thin/dry the atmosphere and hydrographics (opera/firm).
	applyGenreAtmoHydro(world)
	ctx["atmo"], ctx["hydro"] = world.Atmosphere, world.Hydro

	// Population — the port-orientation roll is taken now (firm genre nudges it by pop).
	portRoll := r.TwoD6()
	pop, _ := rs.UWPStep("pop", ctx, r)
	pop, portRoll = firmPopStrip(world, pop, portRoll)
	if pop < 0 {
		pop = 0
	}
	if pop > 15 {
		pop = 15 // population ceiling is F
	}
	pop = capColonyPopulation(world, pop) // hot-star / gravity-band colony cap
	world.Population = pop
	ctx["pop"] = pop

	world.Government, _ = rs.UWPStep("gov", ctx, r)
	ctx["gov"] = world.Government
	world.Law, _ = rs.UWPStep("law", ctx, r)
	ctx["law"] = world.Law

	world.Port = rs.Starport(portRoll)
	ctx["port"] = world.Port

	world.Factions = generateFactions(world.Population, world.Law, r)

	tech := r.D6() + rs.TechDM(ctx)
	if tech < 0 {
		tech = 0
	}
	if activeTechCap != nil && tech > *activeTechCap {
		tech = *activeTechCap // optional config ceiling (Ruby tech_cap)
	}
	if tech > 15 {
		tech = 15
	}
	world.Tech = tech
	if world.Population == 0 { // an uninhabited world has no law, government, or tech
		world.Law, world.Government, world.Tech = 0, 0, 0
	}
	ctx["tech"], ctx["gov"], ctx["law"] = world.Tech, world.Government, world.Law

	world.TradeCodes = rs.TradeCodes(ctx)
	world.Bases = generateBases(world.Port, r)
	world.TravelCode = generateTravelCode(world.Atmosphere, world.Government, world.Law)

	world.Moons = generateMoons(toss(r, 1, 3), world.Size, "", world.Zone, r)

	if world.Population > 0 { // PBG population multiplier (1-9)
		world.PopMultiplier = 1 + r.Intn(9)
	}

	return world
}

func determineZone(au float64, biozone [2]float64) int {
	if au < biozone[0] {
		return -1
	}
	if au > biozone[1] {
		return 1
	}
	return 0
}

// generateFactions mirrors the Ruby model: 1D3 factions (+/- by law level), each a
// type rolled on 2D. (MgT p. 173.)
func generateFactions(pop, law int, r *rng.RNG) []string {
	if pop == 0 {
		return []string{}
	}
	count := r.Roll(1, 3)
	if count > 3 {
		count = 3
	}
	if law == 0 || law == 7 {
		count++
	}
	if law > 9 {
		count--
	}
	if count < 0 {
		count = 0
	}
	types := []string{"O", "O", "O", "O", "F", "F", "M", "M", "N", "N", "S", "S", "P"}
	rolls := []int{r.TwoD6(), r.TwoD6(), r.TwoD6(), r.TwoD6(), r.TwoD6()}
	factions := []string{}
	for i := 0; i < count && i < len(rolls); i++ {
		factions = append(factions, types[rolls[i]])
	}
	return factions
}

// generateBases rolls each base the ruleset allows for this starport, in order
// (naval, scout, depot, way), using the ruleset's comparison direction.
func generateBases(port string, r *rng.RNG) string {
	rs := ruleset()
	bases := ""
	for _, b := range []struct{ kind, letter string }{
		{"naval", "N"}, {"scout", "S"}, {"depot", "D"}, {"way", "W"},
	} {
		if th, ok := rs.BaseThreshold(b.kind, port); ok && rs.BaseMeets(r.TwoD6(), th) {
			bases += b.letter
		}
	}
	if bases == "" {
		bases = "."
	}
	return bases
}

// generateTravelCode mirrors Ruby World#travel_code. T5 leaves zones to the
// referee; this auto-assigns: Red for the most oppressive/controlled worlds
// (law or gov >= 15), Amber for caution.
func generateTravelCode(atmo, gov, law int) string {
	if law >= 15 || gov >= 15 {
		return "R"
	}
	if atmo > 9 || gov == 0 || gov == 7 || gov == 10 || law == 0 || (law >= 9 && law <= 14) {
		return "A"
	}
	return "."
}

// generateMoons mirrors the Ruby Moon class: sizes keyed to the parent (gas-giant
// L/S or planet size - 1d6, possibly negative), radii from the Close (1-14), Ring,
// or Extreme (x25, large GGs only) tables, atmo/hydro by the parent's zone. Moons
// key off their radius (duplicates collapse) and sort by it.
func generateMoons(num, planetSize int, giant string, zone int, r *rng.RNG) []models.Moon {
	closeOrbits := []int{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14}
	ringOrbits := []int{1, 1, 1, 2, 2, 3}

	moons := []models.Moon{}
	for i := 0; i < num; i++ {
		var size int
		switch giant {
		case "L":
			size = toss(r, 2, 4)
		case "S":
			size = toss(r, 2, 6)
		default:
			size = planetSize - r.D6() // may go negative (moonlet)
		}

		orbitRoll := toss(r, 2, i)
		var orbitalRadius int
		switch {
		case size < 1:
			orbitalRadius = ringOrbits[min(toss(r, 1, 1), len(ringOrbits)-1)]
		case orbitRoll == 12 && giant == "L":
			orbitalRadius = closeOrbits[min(toss(r, 2, 0), len(closeOrbits)-1)] * 25
		default:
			orbitalRadius = closeOrbits[min(toss(r, 2, 0), len(closeOrbits)-1)]
		}

		hydro := 0
		if zone == 1 && size != 0 {
			hydro = toss(r, 2, 4)
		} else if zone == 0 && size != 0 {
			hydro = toss(r, 2, 7)
		}

		atmo := 0
		if size != 0 && zone != 0 {
			atmo = toss(r, 2, 7) + size - 4
			if atmo < 0 {
				atmo = 0
			}
		}

		m := models.Moon{
			Orbit:         i,
			OrbitalRadius: orbitalRadius,
			Size:          size,
			Atmo:          atmo,
			Hydro:         hydro,
		}

		// hash-key semantics: a later moon at the same radius replaces the earlier
		replaced := false
		for j := range moons {
			if moons[j].OrbitalRadius == m.OrbitalRadius {
				moons[j] = m
				replaced = true
				break
			}
		}
		if !replaced {
			moons = append(moons, m)
		}
	}

	// sort ascending by radius, renumber
	for i := 1; i < len(moons); i++ {
		for j := i; j > 0 && moons[j-1].OrbitalRadius > moons[j].OrbitalRadius; j-- {
			moons[j-1], moons[j] = moons[j], moons[j-1]
		}
	}
	for i := range moons {
		moons[i].Orbit = i
	}
	return moons
}
