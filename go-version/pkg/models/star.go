package models

import (
	"fmt"
	"math"
)

type StarType string
type StarSize int

const (
	StarTypeO StarType = "O"
	StarTypeB StarType = "B"
	StarTypeA StarType = "A"
	StarTypeF StarType = "F"
	StarTypeG StarType = "G"
	StarTypeK StarType = "K"
	StarTypeM StarType = "M"
	StarTypeD StarType = "D"
)

type StarData struct {
	Example     string
	Temperature int
	Luminosity  float64
	Mass        float64
	Radius      float64
}

var StarChart = map[string]StarData{
	"O9": {"Zeta Ophiuchi", 33000, 55000, 18.0, 7.00},
	"B0": {"Becrux", 30000, 16000, 16.0, 5.70},
	"B2": {"Spica", 22000, 8300, 10.5, 5.10},
	"B5": {"Achernar", 15000, 750, 5.40, 3.70},
	"B8": {"Rigel", 12500, 130, 3.50, 2.70},
	"A0": {"Sirius A", 9500, 63, 2.60, 2.30},
	"A2": {"Fomalhaut", 9000, 40, 2.20, 2.00},
	"A5": {"Altair", 8700, 24, 1.90, 1.80},
	"F0": {"Gamma Virginis", 7400, 9.0, 1.60, 1.50},
	"F2": {".", 7100, 6.3, 1.50, 1.30},
	"F5": {"Procyon A", 6400, 4.0, 1.35, 1.20},
	"G0": {"Alpha Centauri A", 5900, 1.45, 1.08, 1.05},
	"G2": {"The Sun", 5800, 1.00, 1.00, 1.00},
	"G5": {"Mu Cassiopeiae", 5600, 0.70, 0.95, 0.91},
	"G8": {"Tau Ceti", 5300, 0.44, 0.85, 0.87},
	"K0": {"Pollux", 5100, 0.36, 0.83, 0.83},
	"K2": {"Epsilon Eridani", 4830, 0.28, 0.78, 0.79},
	"K5": {"Alpha Centauri B", 4370, 0.18, 0.68, 0.74},
	"M0": {"Gliese 185", 3670, 0.075, 0.47, 0.63},
	"M2": {"Lalande 21185", 3400, 0.03, 0.33, 0.36},
	"M4": {"Ross 128", 3200, 0.0005, 0.20, 0.21},
	"M6": {"Wolf 359", 3000, 0.0002, 0.10, 0.12},
}

var InnerLimit = map[StarType][]float64{
	"O": {16, 13, 10, 8, 6, 1, 0},
	"B": {10, 6.3, 5.0, 4.0, 3.8, 0.6, 0},
	"A": {4, 1, 0.4, 0, 0, 0, 0},
	"F": {4, 1, 0.3, 0.1, 0, 0, 0},
	"G": {3.1, 1, 0.3, 0.1, 0, 0, 0},
	"K": {2.5, 1, 0.3, 0.1, 0, 0, 0},
	"M": {2, 1, 0.3, 0.1, 0, 0, 0},
	"D": {0},
}

var Biozone = map[StarType][][2]float64{
	"O": {{790, 1190}, {630, 950}, {500, 750}, {350, 525}, {235, 350}, {150, 225}, {100, 150}},
	"B": {{500, 700}, {320, 480}, {250, 375}, {200, 300}, {180, 270}, {30, 45}},
	"A": {{200, 300}, {50, 75}, {20, 30}, {5.0, 7.5}, {4.0, 6.0}, {3.1, 4.7}},
	"F": {{200, 300}, {50, 75}, {13, 19}, {2.5, 3.7}, {2.0, 3.0}, {1.6, 2.4}, {0.5, 0.8}},
	"G": {{200, 300}, {50, 75}, {13, 19}, {2.5, 3.7}, {2.0, 3.0}, {1.6, 2.4}, {0.5, 0.8}},
	"K": {{125, 190}, {50, 75}, {13, 19}, {4.0, 5.9}, {1.0, 1.5}, {0.5, 0.6}, {0.2, 0.3}},
	"M": {{100, 150}, {50, 76}, {16, 24}, {5.0, 7.5}, {0, 0}, {0.1, 0.2}, {0.1, 0.1}},
	"D": {{0.03, 0.03}},
}

var StarMass = map[StarType][]float64{
	"O": {90, 60, 40, 25, 20, 16, 16},
	"B": {50, 40, 35, 30, 20, 10},
	"A": {30, 16, 10, 6, 4, 3},
	"F": {15, 13, 8, 2.5, 2.2, 1.9},
	"G": {12, 10, 6, 2.7, 1.8, 1.1, 0.8},
	"K": {15, 12, 6, 3, 2.3, 0.9, 0.5},
	"M": {20, 16, 8, 4, 0.3, 0.2},
	"D": {0.8, 0.8, 0.8, 0.8, 0.8, 0.8},
}

type Star struct {
	Volume      *Volume  `json:"-"`
	Primary     *Star    `json:"-"`
	Companions  []*Star  `json:"companions,omitempty"`
	Orbits      []Orbit  `json:"orbits,omitempty"`
	World       *World   `json:"world,omitempty"`
	StarType    StarType `json:"star_type"`
	StarSize    int      `json:"star_size"`
	Spectral    string   `json:"spectral"`
	StarSubtype string   `json:"star_subtype,omitempty"`
	BodeConstant float64 `json:"bode_constant"`
	TypeDM      int      `json:"type_dm"`
	SizeDM      int      `json:"size_dm"`
	HasGG       bool     `json:"has_gas_giant"`
	// Fractional orbit slot for companions (Ruby @orbit); 0 for primaries.
	Orbit       float64  `json:"orbit"`
}

func (s *Star) Classification() string {
	if s.StarType == "D" {
		return fmt.Sprintf("D%s", s.StarSubtype)
	}
	return fmt.Sprintf("%s%s", s.Spectral, toRoman(s.StarSize))
}

func (s *Star) InnerLimit() float64 {
	limits, ok := InnerLimit[s.StarType]
	if !ok || s.StarSize >= len(limits) {
		return 0
	}
	return limits[s.StarSize]
}

// GetBiozone returns the habitable band and whether this type/size has one. A star
// without a biozone entry reads all its orbits as inner (Ruby's rescue branch).
func (s *Star) GetBiozone() ([2]float64, bool) {
	zones, ok := Biozone[s.StarType]
	if !ok || s.StarSize >= len(zones) {
		return [2]float64{0, 0}, false
	}
	return zones[s.StarSize], true
}

func (s *Star) Luminosity() float64 {
	data, ok := StarChart[s.Spectral]
	if !ok {
		return 1.0
	}
	return data.Luminosity
}

func (s *Star) Mass() float64 {
	masses, ok := StarMass[s.StarType]
	if !ok || s.StarSize >= len(masses) {
		return 0.3
	}
	return masses[s.StarSize]
}

func (s *Star) OuterLimit() float64 {
	return 40 * s.Mass()
}

func (s *Star) SnowLine() float64 {
	return 4.85 * math.Sqrt(s.Luminosity())
}

func (s *Star) OrbitToAU(orbit int) float64 {
	return s.OrbitToAUf(float64(orbit))
}

// OrbitToAUf mirrors Ruby orbit_to_au: inner limit + the Bode progression with the
// product rounded to 0.1 (fractional orbits place companions).
func (s *Star) OrbitToAUf(orbit float64) float64 {
	return s.InnerLimit() + math.Round(s.BodeConstant*math.Pow(2, orbit)*10)/10
}

// AUToOrbit mirrors Ruby au_to_orbit: |log2(au/bode)| (rounded to 0.01) minus the
// inner limit. Uses the primary's Bode constant for companions.
func (s *Star) AUToOrbit(au float64) float64 {
	constant := s.BodeConstant
	if s.Primary != nil {
		constant = s.Primary.BodeConstant
	}
	if au <= 0 || constant <= 0 {
		return 0
	}
	return math.Abs(math.Round(math.Log(au/constant)/math.Log(2)*100)/100) - s.InnerLimit()
}

func (s *Star) HasWorld() bool {
	for _, orbit := range s.Orbits {
		if orbit.IsWorld() {
			return true
		}
	}
	return false
}

func toRoman(n int) string {
	if n == 500 {
		return "D"
	}
	romans := []string{"", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"}
	if n < len(romans) {
		return romans[n]
	}
	return fmt.Sprintf("%d", n)
}