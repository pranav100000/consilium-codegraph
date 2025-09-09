using System;

namespace TestProject
{
    public class Calculator
    {
        private int _value;
        
        public int Value { get; set; }
        
        public Calculator()
        {
            _value = 0;
        }
        
        public int Add(int a, int b)
        {
            return a + b;
        }
        
        public void Reset()
        {
            _value = 0;
            Value = 0;
        }
    }
    
    public interface ICalculator
    {
        int Add(int a, int b);
        void Reset();
    }
    
    public enum Operation
    {
        Add,
        Subtract,
        Multiply,
        Divide
    }
}