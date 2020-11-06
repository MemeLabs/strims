package api

// ServiceRegistry ...
type ServiceRegistry interface {
	RegisterMethod(name string, method interface{})
}
