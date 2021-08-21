package chat

import (
	"errors"
	"strings"
	"sync"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
)

var ErrComboDuplicate = errors.New("user has already participated in combo")

var combos = Combos{}

type comboVariant struct {
	modifiers []string
	count     int
}

type Combos struct {
	lock         sync.Mutex
	emote        string
	count        int
	variants     map[string]*comboVariant
	participants map[string]struct{}
}

func (c *Combos) Transform(msg *chatv1.Message) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	if !isEmoteMessage(msg) {
		c.reset()
		return nil
	}

	emote := msg.Entities.Emotes[0]

	// if the combo was broken by another emote message reset
	if c.emote != emote.Name {
		c.reset()
	}

	if _, ok := c.participants[msg.Nick]; ok {
		return ErrComboDuplicate
	}

	c.emote = emote.Name
	c.count++
	c.participants[msg.Nick] = struct{}{}

	variant := strings.Join(emote.Modifiers, ":")
	if _, ok := c.variants[variant]; !ok {
		c.variants[variant] = &comboVariant{
			modifiers: emote.Modifiers,
			count:     0,
		}
	}
	c.variants[variant].count++

	// if this was the first emote in the combo don't mark a combo yet
	if c.count == 1 {
		return nil
	}

	emote.Combo = uint32(c.count)

	topVariantCount := -1
	for _, v := range c.variants {
		if v.count > topVariantCount {
			topVariantCount = c.count
			emote.Modifiers = v.modifiers
		}
	}

	return nil
}

func (c *Combos) reset() {
	c.emote = ""
	c.count = 0
	c.variants = map[string]*comboVariant{}
	c.participants = map[string]struct{}{}
}

func isEmoteMessage(msg *chatv1.Message) bool {
	if len(msg.Entities.Emotes) != 1 {
		return false
	}
	b := msg.Entities.Emotes[0].Bounds
	return int(b.Start) == 0 && int(b.End) == len(msg.Body)
}
