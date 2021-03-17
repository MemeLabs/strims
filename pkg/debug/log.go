package debug

import (
	"fmt"
	"log"
	"path"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
)

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

var runEveryNCallers = sync.Map{}

func RunEveryNWithStackSkip(n, skip int, fn func()) {
	pc, _, _, _ := runtime.Caller(skip)
	v, _ := runEveryNCallers.LoadOrStore(pc, new(uint64))
	if atomic.AddUint64(v.(*uint64), 1)%uint64(n) == 0 {
		fn()
	}
}

func RunEveryN(n int, fn func()) {
	RunEveryNWithStackSkip(n, 1, fn)
}

// LogEveryN ...
func LogEveryN(n int, msg ...interface{}) {
	RunEveryNWithStackSkip(n, 1, func() { log.Println(msg...) })
}

// LogfEveryN ...
func LogfEveryN(n int, format string, a ...interface{}) {
	RunEveryNWithStackSkip(n, 1, func() { log.Printf(format, a...) })
}
