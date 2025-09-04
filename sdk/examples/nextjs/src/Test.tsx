"use client";

import type { JSX } from 'react';
import { useSDK } from './useSDK';

type Props = {};

export const Test = ({}: Props): JSX.Element => {
    useSDK();

    return <h1>Testing SDK!!!!!!!!!!!!!</h1>;
};
