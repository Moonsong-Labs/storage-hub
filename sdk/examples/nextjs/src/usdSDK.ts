"use client";

import { MspClient } from '@storagehub-sdk/msp-client';
import { useEffect } from 'react';


export const useSDK = () => {
  useEffect(() => {
    console.log(MspClient);
  }, []);
};
