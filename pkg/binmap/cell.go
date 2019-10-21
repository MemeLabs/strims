package binmap

type cell struct {
	left  uint32
	right uint32
}

func (c *cell) Reset() {
	c.left = 0
	c.right = 0
}

func (c *cell) SetLeft(v uint32) {
	c.left = v
}

type ref uint32

func (r ref) MapRef() ref {
	return r &^ 0x1f
}

func (r ref) IsMapRef() bool {
	return r&0x1f == 0
}

type freeCell struct {
	*cell
}

func (c freeCell) NextRef() ref {
	return ref(c.left)
}

func (c freeCell) SetNextRef(r ref) {
	c.left = uint32(r)
}

type mapCell struct {
	*cell
	r ref
}

func (c mapCell) mask() uint32 {
	return uint32(c.r) & 0x1f
}

func (c mapCell) LeftRef() bool {
	return bitmap(c.left).Get(c.mask())
}

func (c mapCell) RightRef() bool {
	return bitmap(c.right).Get(c.mask())
}

func (c mapCell) HasRef() bool {
	m := c.mask()
	return bitmap(c.right).Get(m) || bitmap(c.left).Get(m)
}

func (c mapCell) SetLeftRef(v bool) {
	c.left = uint32(bitmap(c.left).Set(c.mask(), v))
}

func (c mapCell) SetRightRef(v bool) {
	c.right = uint32(bitmap(c.right).Set(c.mask(), v))
}

func (c mapCell) Reset() {
	m := c.mask()
	c.left = uint32(bitmap(c.left).Set(m, false))
	c.right = uint32(bitmap(c.right).Set(m, false))
}

func (c mapCell) Copy(oc mapCell) {
	c.SetLeftRef(oc.LeftRef())
	c.SetRightRef(oc.RightRef())
}

type dataCell struct {
	*cell
}

func (c dataCell) Symmetrical() bool {
	return c.left == c.right
}

func (c dataCell) ResetLeft() {
	c.left = 0
}

func (c dataCell) ResetRight() {
	c.right = 0
}

func (c dataCell) SetLeftRef(r ref) {
	c.left = uint32(r)
}

func (c dataCell) SetRightRef(r ref) {
	c.right = uint32(r)
}

func (c dataCell) LeftRef() ref {
	return ref(c.left)
}

func (c dataCell) RightRef() ref {
	return ref(c.right)
}

func (c dataCell) SetBitmap(b bitmap) {
	c.left = uint32(b)
	c.right = uint32(b)
}

func (c dataCell) SetLeftBitmap(b bitmap) {
	c.left = uint32(b)
}

func (c dataCell) SetRightBitmap(b bitmap) {
	c.right = uint32(b)
}

func (c dataCell) LeftBitmap() bitmap {
	return bitmap(c.left)
}

func (c dataCell) RightBitmap() bitmap {
	return bitmap(c.right)
}

func (c dataCell) Copy(o dataCell) {
	c.left = o.left
	c.right = o.right
}
