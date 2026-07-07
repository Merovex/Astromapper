package builder

import (
	"strings"

	"astromapper/pkg/models"
	"astromapper/pkg/rng"
	"astromapper/pkg/rules"
)

// Package-level generation settings, the Go analog of Ruby's Astromapper.ruleset /
// config globals. Set once before generating a sector (CLI or tests); a single
// sector is generated on one goroutine, so a plain var is sufficient.
var (
	activeRuleset         *rules.Ruleset
	activeSophonts        string // "" or "human" = human-only; "varied" allows alien sophonts
	activeGenre           = "normal"
	activeAlwaysInhabited = true // guarantee a mainworld in the biozone (Ruby always_inhabited)
	activeTechCap         *int   // optional tech-level ceiling (Ruby tech_cap)
)

// SetRuleset selects the active ruleset; SetSophonts the native-life mode;
// SetGenre the realism<->romance stellar slider (firm | normal | opera).
func SetRuleset(rs *rules.Ruleset) { activeRuleset = rs }
func SetSophonts(s string)         { activeSophonts = strings.ToLower(s) }
func SetGenre(g string) {
	if g = strings.ToLower(g); g != "" {
		activeGenre = g
	}
}
func SetAlwaysInhabited(v bool) { activeAlwaysInhabited = v }
func SetTechCap(v *int)         { activeTechCap = v }

// gravityBySize is surface gravity (g) by Size, extended to T5 sizes B-F.
var gravityBySize = []float64{0, 0.05, 0.15, 0.25, 0.35, 0.45, 0.7, 0.9, 1.0, 1.25, 1.4, 1.6, 1.9, 2.2, 2.5, 2.8}

func gravityFor(size int) float64 {
	if size >= 0 && size < len(gravityBySize) {
		return gravityBySize[size]
	}
	return 0
}

func isHotStar(t models.StarType) bool {
	return t == "O" || t == "B" || t == "A" || t == "F"
}

// applyGenreAtmoHydro adjusts Atmosphere/Hydrographics for the realism genres
// (opera/firm), pushing small/odd worlds toward thin/dry. MgT p. 180. (Faithful to
// the Ruby model, including its size 3-4 atmo buckets.)
func applyGenreAtmoHydro(w *models.World) {
	if activeGenre != "opera" && activeGenre != "firm" {
		return
	}
	switch {
	case w.Size < 3 || (w.Size < 4 && w.Atmosphere < 3):
		w.Atmosphere = 0
	case (w.Size == 3 || w.Size == 4) && w.Atmosphere >= 3 && w.Atmosphere <= 5:
		w.Atmosphere = 1
	case (w.Size == 3 || w.Size == 4) && w.Atmosphere > 5:
		w.Atmosphere = 10
	}
	if w.Atmosphere < 2 {
		w.Hydro -= 6
	}
	if w.Atmosphere == 2 || w.Atmosphere == 3 || w.Atmosphere == 11 || w.Atmosphere == 12 {
		w.Hydro -= 4
	}
	if w.Hydro < 0 {
		w.Hydro = 0
	}
}

// firmPopStrip applies the firm-genre population adjustments and returns the
// adjusted port-orientation roll (higher pop -> lower roll -> better port).
func firmPopStrip(w *models.World, pop, portRoll int) (int, int) {
	if activeGenre != "firm" {
		return pop, portRoll
	}
	if w.Size < 3 || w.Size > 9 {
		pop--
	}
	atmoDM := []int{-1, -1, -1, -1, -1, 1, 1, -1, 1, -1, -1, -1, -1, -1, -1, -1}
	if w.Atmosphere >= 0 && w.Atmosphere < len(atmoDM) {
		pop += atmoDM[w.Atmosphere]
	}
	pw := pop
	if pw < 0 {
		pw = 0
	}
	portRoll = portRoll + 7 - pw
	if portRoll < 0 {
		portRoll = 0
	}
	return pop, portRoll
}

// capColonyPopulation caps population at colony size (6) for worlds that can't host
// large native populations: short-lived hot stars, or an uncomfortable gravity band.
func capColonyPopulation(w *models.World, pop int) int {
	if isHotStar(w.Star.StarType) && pop > 6 {
		pop = 6
	}
	if g := gravityFor(w.Size); !(g >= 0.4 && g <= 1.5) && pop > 6 {
		pop = 6
	}
	return pop
}

// ruleset returns the active ruleset, lazily loading the built-in t5 if none is set.
func ruleset() *rules.Ruleset {
	if activeRuleset == nil {
		rs, err := rules.Load("t5", "")
		if err != nil {
			panic("builder: cannot load default ruleset: " + err.Error())
		}
		activeRuleset = rs
	}
	return activeRuleset
}

// ---- climate slot -------------------------------------------------------

func climate(w *models.World, r *rng.RNG) string {
	mod, _ := ruleset().ModuleFor("climate")
	if mod == "none" {
		return "T"
	}
	return climateT5(w, r) // only t5 implemented; unknown names fall back to t5
}

// climateT5 derives climate from the Habitable-Zone variance (Flux), matching Ruby.
func climateT5(w *models.World, r *rng.RNG) string {
	if w.OrbitNumber <= 1 {
		return "Tz"
	}
	variance := []int{-2, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 2}[r.FluxRoll()+6]
	switch {
	case variance <= -1:
		return "H"
	case variance >= 1:
		return "C"
	default:
		return "T"
	}
}

// ---- native slot --------------------------------------------------------

func nativeStatus(w *models.World) string {
	mod, _ := ruleset().ModuleFor("native")
	if mod == "none" {
		return ""
	}
	return nativeStatusT5(w)
}

func nativeStatusT5(w *models.World) string {
	if activeSophonts == "varied" {
		if w.Population >= 7 {
			if w.Atmosphere <= 1 {
				return "Exotic"
			}
			return "Native"
		}
		if w.Population >= 1 && w.Population <= 6 {
			return "Colony"
		}
		return ""
	}
	if w.Population >= 7 {
		return "Settled"
	}
	if w.Population >= 1 && w.Population <= 6 {
		return "Colony"
	}
	return ""
}

// ---- extensions slot ----------------------------------------------------

// buildExtensions runs the ruleset's extensions module (if any), then native status.
// Called as a post-pass once the system's gas-giant and belt counts are known.
func buildExtensions(w *models.World, gasGiants, belts int, r *rng.RNG) {
	mod, _ := ruleset().ModuleFor("extensions")
	if mod != "none" {
		buildExtensionsT5(w, gasGiants, belts, r)
	}
	w.Native = nativeStatus(w)
}

// buildExtensionsT5 computes Ix / Ex / Cx + Resource Units (T5 WorldGen page 435).
func buildExtensionsT5(w *models.World, gasGiants, belts int, r *rng.RNG) {
	tc := w.TradeCodes

	ix := 0
	if w.Port == "A" || w.Port == "B" {
		ix++
	}
	if w.Port == "D" || w.Port == "E" || w.Port == "X" {
		ix--
	}
	if w.Tech >= 10 {
		ix++
	}
	if w.Tech <= 8 {
		ix--
	}
	ix += countAny(tc, "Ag", "Hi", "In", "Ri")
	if w.Population <= 6 {
		ix--
	}
	if hasBase(w.Bases, "N") && hasBase(w.Bases, "S") {
		ix++
	}
	if hasBase(w.Bases, "W") {
		ix++
	}
	w.Ix = ix

	res := r.TwoD6()
	if w.Tech >= 8 {
		res += gasGiants + belts
	}
	lab := w.Population - 1
	if lab < 0 {
		lab = 0
	}
	var inf int
	switch {
	case hasAny(tc, "Ba", "Di", "Lo"):
		inf = 0
	case has(tc, "Ni"):
		inf = r.D6()
	default:
		inf = r.TwoD6() + ix
		if inf < 0 {
			inf = 0
		}
	}
	eff := r.FluxRoll()
	w.Ex = [4]int{res, lab, inf, eff}
	w.RU = nz(res) * nz(lab) * nz(inf) * nz(eff)

	// Cx: Homogeneity, Acceptance, Strangeness, Symbols (each clamped to >= 1).
	homo := min1(w.Population + r.FluxRoll())
	acc := min1(w.Population + ix)
	str := min1(5 + r.FluxRoll())
	sym := min1(w.Tech + r.FluxRoll())
	w.Cx = [4]int{homo, acc, str, sym}

	w.Extended = true
}

func nz(v int) int {
	if v == 0 {
		return 1
	}
	return v
}

func min1(v int) int {
	if v < 1 {
		return 1
	}
	return v
}

func has(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}

func hasAny(list []string, ss ...string) bool {
	for _, s := range ss {
		if has(list, s) {
			return true
		}
	}
	return false
}

func countAny(list []string, ss ...string) int {
	n := 0
	for _, s := range ss {
		if has(list, s) {
			n++
		}
	}
	return n
}

func hasBase(bases, letter string) bool { return strings.Contains(bases, letter) }
