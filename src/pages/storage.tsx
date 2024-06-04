/* eslint-disable prettier/prettier */
import axios from 'axios';
import React, { useEffect, useRef, useState } from 'react';

export const Storage: React.FC = () => {

  const result =  async()=>{
    debugger;
    let result = await axios.get('/api/files');
  }


  useEffect(()=>{
    result();
  },[])


  return (
    <>
      <h1 style={{color: 'red'}}>Hello ...</h1>
    </>
  )
}