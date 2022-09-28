export const material_type_color_map = {
  'Silicon': 'lightblue',
  'Steel': 'grey',
  'Concrete': 'brown'
}

export type RawMaterialsActions = 'BUY' | 'SELL';

export const plant_type_color_map = {
  'Solar': 'yellow',
  'Flare': 'orange',
  'Hydro': 'blue'
}

export type MoveHashboardsActions = "ADD" | "REMOVE";

export const fireSvg = `<svg viewBox="-4 0 36 36">
<text class="text-normal" text-anchor="middle" x="50%" y="20%" dy=".55em">🔥</text>
</svg>`

export const solarSvg = `<svg viewBox="-4 0 36 36">
<text class="text-normal" text-anchor="middle" x="50%" y="20%" dy=".55em">☀️</text>
</svg>`

export const hydroSvg = `<svg viewBox="-4 0 36 36">
<text class="text-normal" text-anchor="middle" x="50%" y="20%" dy=".55em">💧</text>
</svg>`