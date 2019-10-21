package debug

import (
	"fmt"
	"path"
	"runtime"
	"time"
)

var clog colorLogger

type colorLogger struct{}

// func args(c string, vs []interface{}) []interface{} {

// 	return vs
// }

func printLog(color string, vs []interface{}) {
	_, f, l, _ := runtime.Caller(2)
	_, file := path.Split(f)

	vs = append(vs, nil, nil)
	copy(vs[1:], vs)
	vs[0] = fmt.Sprintf(
		"%s%s %s:%d:\u001b[0m",
		color,
		time.Now().Format("2006/01/02 15:04:05.000000"),
		file,
		l,
	)
	vs[len(vs)-1] = "\u001b[0m"

	fmt.Println(vs...)
}

// Black ...
func Black(v ...interface{}) {
	printLog("\u001b[40m", v)
}

// Red ...
func Red(v ...interface{}) {
	printLog("\u001b[41m", v)
}

// Green ...
func Green(v ...interface{}) {
	printLog("\u001b[42m", v)
}

// Yellow ...
func Yellow(v ...interface{}) {
	printLog("\u001b[43m", v)
}

// Blue ...
func Blue(v ...interface{}) {
	printLog("\u001b[44m", v)
}

// Magenta ...
func Magenta(v ...interface{}) {
	printLog("\u001b[45m", v)
}

// Cyan ...
func Cyan(v ...interface{}) {
	printLog("\u001b[46m", v)
}
