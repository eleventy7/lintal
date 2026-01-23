public class Input {
    public void method() {
        // Binary operators on new line - should move to end of previous line
        int sum = 1 +
            2;
        int diff = 10 -
            3;
        int product = 5 *
            4;

        // Comparison operators on new line
        boolean test = sum >
            diff;

        // Logical operators on new line
        boolean logic = true &&
            false;

        // Ternary operator on new line
        int result = test ?
            1 : 2;

        // Assignment compound operators (not in default tokens, unchanged)
        sum
            += 10;
    }
}
