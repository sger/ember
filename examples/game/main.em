; main.ember
import "player.em"
import "enemy.em"

Player.create
25 Player.damage
dup print               ; 75

use Enemy.*
dragon print            ; 200
goblin print            ; 30
