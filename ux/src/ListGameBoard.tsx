import { Typography } from '@mui/material';
import React from 'react';
import { GameBoard } from './Types/Gameboard';

export function ListGameBoard(props: { g: GameBoard | null; }) {
  return props.g &&
    <Typography component={"textarea"}

      style={{ width: "100%", maxHeight: "75vh" }}
    >
      {JSON.stringify(props.g, null, 2)}
    </Typography>
}
