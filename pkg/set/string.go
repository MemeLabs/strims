package set

func NewString(size int) *String {
	return &String{
		values: make(map[string]struct{}, size),
	}
}

type String struct {
	values map[string]struct{}
}

func (s *String) Insert(v string) {
	if _, ok := s.values[v]; !ok {
		s.values[v] = struct{}{}
	}
}

func (s *String) Slice() []string {
	vs := make([]string, 0, len(s.values))
	for v := range s.values {
		vs = append(vs, v)
	}
	return vs
}
