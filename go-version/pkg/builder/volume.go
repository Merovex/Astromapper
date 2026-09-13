package builder

import (
	"astromapper/pkg/models"
	"astromapper/pkg/rng"
	"fmt"
	"math"
)

func BuildVolume(col, row int, names []string, r *rng.RNG) *models.Volume {
	volume := &models.Volume{
		Column: col,
		Row:    row,
	}
	
	if len(names) > 0 {
		volume.Name = names[r.Intn(len(names))]
	} else {
		volume.Name = fmt.Sprintf("%02d%02d", col, row)
	}
	
	volume.Star = BuildStar(volume, nil, 0, r)
	
	companionRoll := r.TwoD6()
	companionCounts := []int{0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2}
	numCompanions := 0
	if companionRoll < len(companionCounts) {
		numCompanions = companionCounts[companionRoll]
	}
	
	for i := 0; i < numCompanions; i++ {
		companion := BuildStar(volume, volume.Star, i, r)
		volume.Star.Companions = append(volume.Star.Companions, companion)
		insertCompanion(volume.Star, companion)
	}

	return volume
}

// insertCompanion mirrors Ruby Star#companions=: place an 'S' orbit for the
// companion, nil out the GURPS forbidden zone (0.67x-3x its distance, Space 4e
// p.107), fill the gaps with empty orbits, and re-prune/renumber.
func insertCompanion(primary *models.Star, companion *models.Star) {
	orbitF := math.Abs(companion.Orbit)
	companionAU := primary.OrbitToAUf(orbitF)

	innerOrbit := int(math.Floor(primary.AUToOrbit(companionAU * 0.67)))
	outerOrbit := int(math.Ceil(primary.AUToOrbit(companionAU * 3)))

	slots := make([]models.Orbit, len(primary.Orbits))
	copy(slots, primary.Orbits)
	for x := max(innerOrbit, 0); x <= outerOrbit && x < len(slots); x++ {
		slots[x] = nil
	}

	insertIdx := int(math.Trunc(orbitF - 1))
	if insertIdx < 0 {
		insertIdx = 0
	}
	for len(slots) <= insertIdx {
		slots = append(slots, nil)
	}
	slots[insertIdx] = &models.Companion{
		BaseOrbit: models.BaseOrbit{
			Star:        primary,
			OrbitNumber: int(math.Round(orbitF)),
			AU:          companionAU,
			Kid:         models.OrbitCompanion,
		},
		CompanionStar: companion,
	}

	orbits := make([]models.Orbit, 0, len(slots))
	for i, s := range slots {
		if s == nil {
			s = &models.EmptyOrbit{BaseOrbit: models.BaseOrbit{
				Star:        primary,
				OrbitNumber: i,
				AU:          primary.OrbitToAU(i),
				Kid:         models.OrbitEmpty,
			}}
		}
		orbits = append(orbits, s)
	}
	primary.Orbits = orbits
	pruneOrbits(primary)
}