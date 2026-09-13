package models

import "fmt"

type Moon struct {
	Planet        *BaseOrbit `json:"-"`
	Orbit         int        `json:"orbit"`
	OrbitalRadius int        `json:"orbital_radius"` // Distance in planetary radii
	// Size may be negative for the tiniest moonlets (rendered 'S'; 0 renders 'R').
	Size  int `json:"size"`
	Atmo  int `json:"atmosphere"`
	Hydro int `json:"hydrographics"`
}

// UWP mirrors Ruby Moon#uwp: X + size + atmo + hydro + 000 (no dash/tech).
func (m *Moon) UWP() string {
	size := toHex(m.Size)
	if m.Size < 0 {
		size = "S"
	} else if m.Size == 0 {
		size = "R"
	}
	return fmt.Sprintf("X%s%s%s000", size, toHex(m.Atmo), toHex(m.Hydro))
}

// ToASCII mirrors the Ruby moon detail row: indented "/  NNN rad. UWP".
func (m *Moon) ToASCII() string {
	return fmt.Sprintf("\n%28s/  %3d rad. %s", "", m.OrbitalRadius, m.UWP())
}
