#ifndef _TB_ATTACK_INTERFACE
#define _TB_ATTACK_INTERFACE

#ifdef __cplusplus
#include <cstdint>
#else
#include <stdbool.h>
#include <stdint.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

uint64_t ven_tb_knight_attacks(unsigned square);
uint64_t ven_tb_king_attacks(unsigned square);
uint64_t ven_tb_rook_attacks(unsigned square, uint64_t occ);
uint64_t ven_tb_bishop_attacks(unsigned square, uint64_t occ);
uint64_t ven_tb_queen_attacks(unsigned square, uint64_t occ);
uint64_t ven_tb_pawn_attacks(unsigned square, bool color);

#ifdef __cplusplus
}
#endif

#endif
