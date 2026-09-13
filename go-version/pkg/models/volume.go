package models

import (
	"fmt"
)

type Volume struct {
	Name      string `json:"name"`
	Column    int    `json:"column"`
	Row       int    `json:"row"`
	Star      *Star  `json:"star,omitempty"`
	GasGiant  string `json:"gas_giant,omitempty"`
}

func (v *Volume) Location() string {
	return fmt.Sprintf("%02d%02d", v.Column, v.Row)
}

func (v *Volume) IsEmpty() bool {
	return v.Star == nil || v.Star.World == nil
}

func (v *Volume) ToASCII() string {
	if v.IsEmpty() {
		return ""
	}
	
	w := v.Star.World
	tradeCodes := joinStrings(w.TradeCodes, " ")
	factions := joinStrings(w.Factions, " ")
	
	bases := w.Bases
	if bases == "" {
		bases = "."
	}
	
	travel := ".."
	switch w.TravelCode {
	case "R":
		travel = "RZ"
	case "A":
		travel = "AZ"
	}

	summary := fmt.Sprintf("%-8s %-9s %-4s %-5s %-2s %-11s %-12s %-13s %-13s %s",
		v.Location(),
		w.GetUWP(),
		w.Temperature,
		bases,
		travel,
		tradeCodes,
		factions,
		v.Star.Crib(),
		v.Star.OrbitsCrib(),
		v.Name)

	if ext := w.Extensions(); ext != "" {
		summary += "  " + ext
	}
	if w.Native != "" {
		summary += "  " + w.Native
	}

	summary += v.Star.OrbitsToASCII()
	return summary
}

// ToTab renders one T5 Second Survey row (tab-delimited). Mirrors Ruby Volume#to_tab.
func (v *Volume) ToTab(sectorName, allegiance string) string {
	w := v.Star.World
	ss := string(rune('A' + ((v.Row-1)/10)*4 + ((v.Column-1)/8)))

	belts, gg, worlds := 0, 0, 0
	for _, o := range v.Star.Orbits {
		switch o.GetKid() {
		case OrbitBelt:
			belts++
		case OrbitGasGiant:
			gg++
		case OrbitWorld:
			worlds++
		}
	}
	if worlds < 1 {
		worlds = 1
	}
	pbg := fmt.Sprintf("%d%d%d", w.PopMultiplier, cap9(belts), cap9(gg))

	zone := ""
	if w.TravelCode == "A" {
		zone = "A"
	} else if w.TravelCode == "R" {
		zone = "R"
	}

	ix, ex, cx := "", "", ""
	if w.Extended {
		ix = fmt.Sprintf("{ %d }", w.Ix)
		ex = fmt.Sprintf("(%s%s%s%+d)", toHex(w.Ex[0]), toHex(w.Ex[1]), toHex(w.Ex[2]), w.Ex[3])
		cx = fmt.Sprintf("[%s%s%s%s]", toHex(w.Cx[0]), toHex(w.Cx[1]), toHex(w.Cx[2]), toHex(w.Cx[3]))
	}

	t5bases := w.Bases
	if t5bases == "." {
		t5bases = ""
	}

	fields := []string{
		sectorName, ss, v.Location(), v.Name, w.GetUWP(), t5bases,
		joinStrings(w.TradeCodes, " "), zone, pbg, allegiance,
		v.Star.t5Stars(), ix, ex, cx, "",
		fmt.Sprintf("%d", worlds), fmt.Sprintf("%d", w.RU),
	}
	return joinStrings(fields, "\t")
}

func cap9(n int) int {
	if n > 9 {
		return 9
	}
	return n
}

// t5Stars is the space-joined classifications of the primary and its companions.
func (s *Star) t5Stars() string {
	stars := []string{s.Classification()}
	for _, comp := range s.Companions {
		stars = append(stars, comp.Classification())
	}
	return joinStrings(stars, " ")
}

func (s *Star) Crib() string {
	stars := []string{s.Classification()}
	for _, comp := range s.Companions {
		stars = append(stars, comp.Classification())
	}
	
	return joinStrings(stars, "/")
}

func (s *Star) OrbitsCrib() string {
	orbits := ""
	for _, o := range s.Orbits {
		orbits += string(o.GetKid())
	}
	return orbits
}

func (s *Star) OrbitsToASCII() string {
	if len(s.Orbits) == 0 {
		return ""
	}
	result := "\n"
	for i, orbit := range s.Orbits {
		if i > 0 {
			result += "\n"
		}
		result += orbit.ToASCII()
	}
	result += "\n"
	return result
}

func joinStrings(strs []string, sep string) string {
	if len(strs) == 0 {
		return ""
	}
	result := strs[0]
	for i := 1; i < len(strs); i++ {
		result += sep + strs[i]
	}
	return result
}